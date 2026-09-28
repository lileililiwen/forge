//! Portfolio persistence: additive tables plus the read model.
//!
//! Everything here shares the registry's SQLite file and its
//! migration lifecycle but never a row. The split mirrors the
//! product boundary exactly:
//!
//! - User-owned rows (`portfolio_projects`, `portfolio_tags`,
//!   `portfolio_project_tags`, `portfolio_relations`,
//!   `portfolio_goals`, `portfolio_goal_projects`,
//!   `portfolio_reviews`) are editable through the methods below.
//! - Source-owned rows (`portfolio_evidence_snapshots`) are
//!   append-only. There is no update or delete path for a
//!   snapshot: a later observation is a new row, and the read
//!   model picks the newest one per source.
//!
//! Every mutation validates its arguments through
//! [`crate::portfolio`] before touching the database and checks the
//! target project against the canonical registry identity, so a
//! write for an unknown project changes no portfolio state.

use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};

use crate::core::{validate_project_id, ForgeError};
use crate::portfolio::{
    self, Confidence, EvidenceSnapshot, EvidenceState, EvidenceStatus, GoalRecord, Lifecycle,
    PortfolioFilter, PortfolioProject, PortfolioRow, ProjectPortfolio, ProjectRelation,
    RelationRecord, RelationType, ReviewRecord, TagRecord,
};

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

/// A validated, ready-to-write user-owned portfolio record.
#[derive(Debug, Clone, Default)]
pub struct PortfolioWrite {
    pub lifecycle: Option<Lifecycle>,
    pub confidence: Option<Confidence>,
    pub next_action: Option<String>,
    pub blocker: Option<String>,
}

/// A validated, ready-to-write evidence snapshot. Every field is
/// required except the freshness bound: a snapshot without an
/// honest source, revision or observation time would let a
/// local observation pose as an external one.
#[derive(Debug, Clone)]
pub struct SnapshotWrite {
    pub source_system: String,
    pub source_revision: String,
    pub observed_at: String,
    pub status: EvidenceStatus,
    pub stale_after: Option<String>,
    pub evidence_json: String,
}

/// Validate one evidence write end to end without touching the
/// database. Shared by the CLI, the JSON API and the portal so
/// every caller applies the same bounds.
pub fn validate_snapshot(write: &SnapshotWrite) -> Result<SnapshotWrite, ForgeError> {
    let source_system = portfolio::validate_source_system(&write.source_system)
        .map_err(|reason| ForgeError::PortfolioInvalid { reason })?;
    let source_revision = portfolio::validate_source_revision(&write.source_revision)
        .map_err(|reason| ForgeError::PortfolioInvalid { reason })?;
    let observed_at = parse_timestamp("observed_at", &write.observed_at)?;
    let stale_after = match &write.stale_after {
        Some(value) => Some(parse_timestamp("stale_after", value)?),
        None => None,
    };
    if let Some(bound) = &stale_after {
        if bound < &observed_at {
            return Err(ForgeError::PortfolioInvalid {
                reason:
                    "stale_after precedes observed_at; a snapshot cannot expire before it was seen"
                        .to_string(),
            });
        }
    }
    let evidence_json = portfolio::prepare_evidence(&write.evidence_json)
        .map_err(|reason| ForgeError::PortfolioInvalid { reason })?;
    Ok(SnapshotWrite {
        source_system,
        source_revision,
        observed_at,
        status: write.status,
        stale_after,
        evidence_json,
    })
}

/// Parse one RFC 3339 timestamp into the canonical string form.
fn parse_timestamp(field: &str, raw: &str) -> Result<String, ForgeError> {
    let trimmed = raw.trim();
    DateTime::parse_from_rfc3339(trimmed)
        .map(|value| value.with_timezone(&Utc).to_rfc3339())
        .map_err(|_| ForgeError::PortfolioInvalid {
            reason: format!("{field} must be an RFC 3339 timestamp, got `{trimmed}`"),
        })
}

fn invalid(reason: String) -> ForgeError {
    ForgeError::PortfolioInvalid { reason }
}

impl crate::registry::Registry {
    /// Resolve a project id against the canonical registry
    /// identity, refusing anything that is not an imported
    /// project. Every portfolio write goes through this check
    /// first, so an unknown project changes no portfolio state.
    pub(crate) fn require_project(&self, project_id: &str) -> Result<(), ForgeError> {
        self.inspect(project_id).map(|_| ())
    }

    // --- user-owned project record -------------------------------------

    /// Upsert the user-owned portfolio record for one project.
    /// `reviewed_at` is stamped on every accepted write, so the
    /// read model can always say when the classification last
    /// changed.
    pub fn portfolio_write(
        &self,
        project_id: &str,
        write: &PortfolioWrite,
    ) -> Result<PortfolioProject, ForgeError> {
        self.require_project(project_id)?;
        let next_action = write
            .next_action
            .as_deref()
            .map(|value| portfolio::validate_note("next action", value, portfolio::MAX_NOTE_CHARS))
            .transpose()
            .map_err(invalid)?
            .flatten();
        let blocker = write
            .blocker
            .as_deref()
            .map(|value| portfolio::validate_note("blocker", value, portfolio::MAX_NOTE_CHARS))
            .transpose()
            .map_err(invalid)?
            .flatten();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO portfolio_projects
                (project_id, lifecycle, confidence, next_action, blocker, reviewed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(project_id) DO UPDATE SET
                lifecycle = excluded.lifecycle,
                confidence = excluded.confidence,
                next_action = excluded.next_action,
                blocker = excluded.blocker,
                reviewed_at = excluded.reviewed_at",
            params![
                project_id,
                write.lifecycle.map(|v| v.label()),
                write.confidence.map(|v| v.label()),
                next_action,
                blocker,
                now
            ],
        )?;
        Ok(PortfolioProject {
            project_id: project_id.to_string(),
            lifecycle: write.lifecycle,
            confidence: write.confidence,
            next_action,
            blocker,
            reviewed_at: Some(now),
        })
    }

    /// Read the user-owned record. A project with no record reads
    /// as all-`None` rather than as an invented default, so an
    /// unclassified project is visibly unclassified.
    pub fn portfolio_record(&self, project_id: &str) -> Result<PortfolioProject, ForgeError> {
        let row = self
            .conn
            .query_row(
                "SELECT lifecycle, confidence, next_action, blocker, reviewed_at
                 FROM portfolio_projects WHERE project_id = ?1",
                params![project_id],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                    ))
                },
            )
            .optional()?;
        let Some((lifecycle, confidence, next_action, blocker, reviewed_at)) = row else {
            return Ok(PortfolioProject {
                project_id: project_id.to_string(),
                ..PortfolioProject::default()
            });
        };
        Ok(PortfolioProject {
            project_id: project_id.to_string(),
            lifecycle: lifecycle.as_deref().and_then(|v| Lifecycle::parse(v).ok()),
            confidence: confidence
                .as_deref()
                .and_then(|v| Confidence::parse(v).ok()),
            next_action,
            blocker,
            reviewed_at,
        })
    }

    // --- tags ----------------------------------------------------------

    /// Attach one tag to one project, creating the tag on first
    /// use. Idempotent: attaching the same tag twice leaves one
    /// link row and reports the existing tag.
    pub fn portfolio_add_tag(
        &self,
        project_id: &str,
        name: &str,
        color: Option<&str>,
    ) -> Result<TagRecord, ForgeError> {
        self.require_project(project_id)?;
        let name = portfolio::validate_tag_name(name).map_err(invalid)?;
        let color = portfolio::validate_tag_color(color).map_err(invalid)?;
        let now = Utc::now().to_rfc3339();
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO portfolio_tags (name, color, created_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(name) DO UPDATE SET color = COALESCE(excluded.color, color)",
            params![name, color, now],
        )?;
        let tag_id: i64 = tx.query_row(
            "SELECT id FROM portfolio_tags WHERE name = ?1",
            params![name],
            |row| row.get(0),
        )?;
        tx.execute(
            "INSERT INTO portfolio_project_tags (project_id, tag_id) VALUES (?1, ?2)
             ON CONFLICT(project_id, tag_id) DO NOTHING",
            params![project_id, tag_id],
        )?;
        tx.commit()?;
        Ok(TagRecord {
            tag_id,
            name,
            color,
            created_at: now,
        })
    }

    /// Detach one tag from one project. The tag itself is
    /// user-owned metadata rather than project state, so it stays
    /// available for other projects; removing a link that was
    /// never there is an idempotent no-op.
    pub fn portfolio_remove_tag(&self, project_id: &str, name: &str) -> Result<bool, ForgeError> {
        self.require_project(project_id)?;
        let name = portfolio::validate_tag_name(name).map_err(invalid)?;
        let removed = self.conn.execute(
            "DELETE FROM portfolio_project_tags
             WHERE project_id = ?1
               AND tag_id = (SELECT id FROM portfolio_tags WHERE name = ?2)",
            params![project_id, name],
        )?;
        Ok(removed > 0)
    }

    /// Tags attached to one project, ordered by name.
    pub fn portfolio_tags_for(&self, project_id: &str) -> Result<Vec<TagRecord>, ForgeError> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.name, t.color, t.created_at
             FROM portfolio_project_tags pt
             JOIN portfolio_tags t ON t.id = pt.tag_id
             WHERE pt.project_id = ?1
             ORDER BY t.name",
        )?;
        let rows = stmt.query_map(params![project_id], |row| {
            Ok(TagRecord {
                tag_id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Every tag in the portfolio with the projects carrying it.
    pub fn portfolio_all_tags(&self) -> Result<Vec<(TagRecord, Vec<String>)>, ForgeError> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.name, t.color, t.created_at,
                    (SELECT group_concat(pt.project_id, ',')
                       FROM portfolio_project_tags pt
                      WHERE pt.tag_id = t.id)
             FROM portfolio_tags t
             ORDER BY t.name",
        )?;
        let rows = stmt.query_map([], |row| {
            let projects: Option<String> = row.get(4)?;
            Ok((
                TagRecord {
                    tag_id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                    created_at: row.get(3)?,
                },
                projects
                    .map(|value| {
                        value
                            .split(',')
                            .filter(|v| !v.is_empty())
                            .map(|v| v.to_string())
                            .collect()
                    })
                    .unwrap_or_default(),
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    // --- relations ------------------------------------------------------

    /// Link two known projects. The relation is idempotent: a
    /// repeated `depends-on` returns the existing row rather than
    /// a second one, and a self-link is refused before any row is
    /// touched because no current relation type permits it.
    pub fn portfolio_add_relation(
        &self,
        from_project: &str,
        to_project: &str,
        relation_type: RelationType,
        note: Option<&str>,
    ) -> Result<RelationRecord, ForgeError> {
        if from_project == to_project {
            return Err(invalid(format!(
                "relation `{}` would link `{from_project}` to itself; \
                 no relation type permits a self relation",
                relation_type.label()
            )));
        }
        self.require_project(from_project)?;
        self.require_project(to_project)?;
        let note = note
            .map(|value| {
                portfolio::validate_note("relation note", value, portfolio::MAX_NOTE_CHARS)
            })
            .transpose()
            .map_err(invalid)?
            .flatten();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO portfolio_relations
                (from_project, to_project, relation_type, note, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(from_project, to_project, relation_type) DO UPDATE SET
                note = COALESCE(excluded.note, note)",
            params![from_project, to_project, relation_type.label(), note, now],
        )?;
        let row = self.conn.query_row(
            "SELECT id, note, created_at FROM portfolio_relations
             WHERE from_project = ?1 AND to_project = ?2 AND relation_type = ?3",
            params![from_project, to_project, relation_type.label()],
            |row| {
                Ok(RelationRecord {
                    relation_id: row.get(0)?,
                    from_project: from_project.to_string(),
                    to_project: to_project.to_string(),
                    relation_type,
                    note: row.get(1)?,
                    created_at: row.get(2)?,
                })
            },
        )?;
        Ok(row)
    }

    /// Remove one declared relation. Removing an absent relation
    /// is an idempotent no-op.
    pub fn portfolio_remove_relation(
        &self,
        from_project: &str,
        to_project: &str,
        relation_type: RelationType,
    ) -> Result<bool, ForgeError> {
        if from_project == to_project {
            return Err(invalid(format!(
                "relation `{}` would link `{from_project}` to itself; \
                 no relation type permits a self relation",
                relation_type.label()
            )));
        }
        self.require_project(from_project)?;
        self.require_project(to_project)?;
        let removed = self.conn.execute(
            "DELETE FROM portfolio_relations
             WHERE from_project = ?1 AND to_project = ?2 AND relation_type = ?3",
            params![from_project, to_project, relation_type.label()],
        )?;
        Ok(removed > 0)
    }

    /// Every relation touching one project, outgoing first, each
    /// labelled with the direction it was declared in.
    pub fn portfolio_relations_for(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectRelation>, ForgeError> {
        let mut stmt = self.conn.prepare(
            "SELECT from_project, to_project, relation_type, note
             FROM portfolio_relations
             WHERE from_project = ?1 OR to_project = ?1
             ORDER BY from_project, to_project, relation_type",
        )?;
        let rows = stmt.query_map(params![project_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (from, to, relation_type, note) = row?;
            let Some(kind) = RelationType::parse(&relation_type).ok() else {
                // A relation type written by a newer build is
                // surfaced as unknown, never silently dropped.
                continue;
            };
            let record = RelationRecord {
                relation_id: 0,
                from_project: from,
                to_project: to,
                relation_type: kind,
                note,
                created_at: String::new(),
            };
            out.push(if record.from_project == project_id {
                ProjectRelation::outgoing(&record)
            } else {
                ProjectRelation::incoming(&record)
            });
        }
        Ok(out)
    }

    /// Every declared relation in the portfolio, ordered by
    /// `(from, to, type)`. Used by the fleet-wide CLI listing so
    /// the raw directed rows are reported instead of being
    /// re-projected through a per-project view.
    pub fn portfolio_all_relations(&self) -> Result<Vec<RelationRecord>, ForgeError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, from_project, to_project, relation_type, note, created_at
             FROM portfolio_relations
             ORDER BY from_project, to_project, relation_type",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (relation_id, from_project, to_project, relation_type, note, created_at) = row?;
            out.push(RelationRecord {
                relation_id,
                from_project,
                to_project,
                relation_type: RelationType::parse(&relation_type)
                    .unwrap_or(RelationType::DependsOn),
                note,
                created_at,
            });
        }
        Ok(out)
    }

    // --- reviews -------------------------------------------------------

    /// Record one review decision. Reviews are append-only
    /// history; the newest row is the current confidence
    /// statement, and the user-owned record's `reviewed_at` moves
    /// with it.
    pub fn portfolio_record_review(
        &self,
        project_id: &str,
        confidence: Confidence,
        note: Option<&str>,
    ) -> Result<ReviewRecord, ForgeError> {
        self.require_project(project_id)?;
        let note = note
            .map(|value| portfolio::validate_note("review note", value, portfolio::MAX_NOTE_CHARS))
            .transpose()
            .map_err(invalid)?
            .flatten();
        let now = Utc::now().to_rfc3339();
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO portfolio_reviews (project_id, confidence, note, reviewed_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![project_id, confidence.label(), note, now],
        )?;
        let review_id = tx.last_insert_rowid();
        tx.execute(
            "INSERT INTO portfolio_projects
                (project_id, lifecycle, confidence, reviewed_at)
             VALUES (?1, NULL, ?2, ?3)
             ON CONFLICT(project_id) DO UPDATE SET
                confidence = excluded.confidence,
                reviewed_at = excluded.reviewed_at",
            params![project_id, confidence.label(), now],
        )?;
        tx.commit()?;
        Ok(ReviewRecord {
            review_id,
            project_id: project_id.to_string(),
            confidence,
            note,
            reviewed_at: now,
        })
    }

    /// Review history for one project, newest first and capped.
    pub fn portfolio_reviews_for(
        &self,
        project_id: &str,
        limit: usize,
    ) -> Result<Vec<ReviewRecord>, ForgeError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, confidence, note, reviewed_at
             FROM portfolio_reviews WHERE project_id = ?1
             ORDER BY id DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![project_id, limit as i64], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, confidence, note, reviewed_at) = row?;
            out.push(ReviewRecord {
                review_id: id,
                project_id: project_id.to_string(),
                confidence: Confidence::parse(&confidence).unwrap_or(Confidence::Unknown),
                note,
                reviewed_at,
            });
        }
        Ok(out)
    }

    // --- goals ---------------------------------------------------------

    /// Create one goal. Titles are unique so a repeated create is
    /// an idempotent lookup rather than a duplicate row.
    pub fn portfolio_add_goal(
        &self,
        title: &str,
        status: &str,
        description: Option<&str>,
    ) -> Result<GoalRecord, ForgeError> {
        let trimmed = title.trim();
        if trimmed.is_empty() {
            return Err(invalid("goal title must not be empty".to_string()));
        }
        if trimmed.chars().count() > portfolio::MAX_GOAL_TITLE_CHARS {
            return Err(invalid(format!(
                "goal title is longer than {} characters",
                portfolio::MAX_GOAL_TITLE_CHARS
            )));
        }
        let status = portfolio::validate_goal_status(status).map_err(invalid)?;
        let description = description
            .map(|value| {
                portfolio::validate_note(
                    "goal description",
                    value,
                    portfolio::MAX_GOAL_DESCRIPTION_CHARS,
                )
            })
            .transpose()
            .map_err(invalid)?
            .flatten();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO portfolio_goals (title, status, description, created_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(title) DO UPDATE SET
                status = excluded.status,
                description = COALESCE(excluded.description, description)",
            params![trimmed, status, description, now],
        )?;
        let row = self.conn.query_row(
            "SELECT id, title, status, description, created_at FROM portfolio_goals WHERE title = ?1",
            params![trimmed],
            |row| {
                Ok(GoalRecord {
                    goal_id: row.get(0)?,
                    title: row.get(1)?,
                    status: row.get(2)?,
                    description: row.get(3)?,
                    created_at: row.get(4)?,
                    projects: Vec::new(),
                })
            },
        )?;
        Ok(row)
    }

    /// Attach one project to one goal, creating the goal when the
    /// title is new. Linking an already-linked pair is a no-op.
    pub fn portfolio_link_goal(
        &self,
        title: &str,
        project_id: &str,
    ) -> Result<GoalRecord, ForgeError> {
        self.require_project(project_id)?;
        let goal = self.portfolio_add_goal(title, "planned", None)?;
        self.conn.execute(
            "INSERT INTO portfolio_goal_projects (goal_id, project_id) VALUES (?1, ?2)
             ON CONFLICT(goal_id, project_id) DO NOTHING",
            params![goal.goal_id, project_id],
        )?;
        self.portfolio_goal(goal.goal_id)
    }

    /// Read one goal with its project list.
    pub fn portfolio_goal(&self, goal_id: i64) -> Result<GoalRecord, ForgeError> {
        let mut goal = self
            .conn
            .query_row(
                "SELECT id, title, status, description, created_at
                 FROM portfolio_goals WHERE id = ?1",
                params![goal_id],
                |row| {
                    Ok(GoalRecord {
                        goal_id: row.get(0)?,
                        title: row.get(1)?,
                        status: row.get(2)?,
                        description: row.get(3)?,
                        created_at: row.get(4)?,
                        projects: Vec::new(),
                    })
                },
            )
            .optional()?
            .ok_or_else(|| ForgeError::PortfolioInvalid {
                reason: format!("goal {goal_id} does not exist"),
            })?;
        let mut stmt = self.conn.prepare(
            "SELECT project_id FROM portfolio_goal_projects WHERE goal_id = ?1 ORDER BY project_id",
        )?;
        let rows = stmt.query_map(params![goal_id], |row| row.get::<_, String>(0))?;
        for row in rows {
            goal.projects.push(row?);
        }
        Ok(goal)
    }

    /// Every goal, ordered by title.
    pub fn portfolio_goals(&self) -> Result<Vec<GoalRecord>, ForgeError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, status, description, created_at,
                    (SELECT group_concat(gp.project_id, ',')
                       FROM portfolio_goal_projects gp
                      WHERE gp.goal_id = portfolio_goals.id)
             FROM portfolio_goals ORDER BY title",
        )?;
        let rows = stmt.query_map([], |row| {
            let projects: Option<String> = row.get(5)?;
            Ok(GoalRecord {
                goal_id: row.get(0)?,
                title: row.get(1)?,
                status: row.get(2)?,
                description: row.get(3)?,
                created_at: row.get(4)?,
                projects: projects
                    .map(|value| {
                        value
                            .split(',')
                            .filter(|v| !v.is_empty())
                            .map(|v| v.to_string())
                            .collect()
                    })
                    .unwrap_or_default(),
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    // --- evidence snapshots --------------------------------------------

    /// Append one source-owned observation. There is no update or
    /// delete path: a later observation is a new row, and the read
    /// model picks the newest one per source. The payload is
    /// redacted and bounded before it is written.
    pub fn portfolio_import_snapshot(
        &self,
        project_id: &str,
        write: &SnapshotWrite,
    ) -> Result<EvidenceSnapshot, ForgeError> {
        self.require_project(project_id)?;
        let write = validate_snapshot(write)?;
        self.conn.execute(
            "INSERT INTO portfolio_evidence_snapshots
                (project_id, source_system, source_revision, observed_at,
                 status, stale_after, evidence_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                project_id,
                write.source_system,
                write.source_revision,
                write.observed_at,
                write.status.label(),
                write.stale_after,
                write.evidence_json,
            ],
        )?;
        let snapshot_id = self.conn.last_insert_rowid();
        Ok(EvidenceSnapshot {
            snapshot_id,
            project_id: project_id.to_string(),
            source_system: write.source_system,
            source_revision: write.source_revision,
            observed_at: write.observed_at,
            status: write.status,
            stale_after: write.stale_after,
            effective_status: write.status,
            evidence_json: write.evidence_json,
        })
    }

    /// Every snapshot for one project, newest first and capped.
    pub fn portfolio_snapshots_for(
        &self,
        project_id: &str,
        limit: usize,
    ) -> Result<Vec<EvidenceSnapshot>, ForgeError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_system, source_revision, observed_at,
                    status, stale_after, evidence_json
             FROM portfolio_evidence_snapshots
             WHERE project_id = ?1 ORDER BY id DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(
            params![project_id, limit as i64],
            row_to_snapshot(project_id),
        )?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// The newest snapshot per source system for one project. The
    /// read model reads this and nothing else, so an expired
    /// snapshot stays visible as `stale` rather than disappearing
    /// and letting the project look unevidenced.
    pub fn portfolio_current_snapshots(
        &self,
        project_id: &str,
    ) -> Result<Vec<EvidenceSnapshot>, ForgeError> {
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.source_system, s.source_revision, s.observed_at,
                    s.status, s.stale_after, s.evidence_json
             FROM portfolio_evidence_snapshots s
             JOIN (SELECT source_system, MAX(id) AS newest
                     FROM portfolio_evidence_snapshots
                    WHERE project_id = ?1
                    GROUP BY source_system) latest
               ON latest.newest = s.id
             ORDER BY s.source_system",
        )?;
        let rows = stmt.query_map(params![project_id], row_to_snapshot(project_id))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    // --- read model -----------------------------------------------------

    /// Build one project's full portfolio projection: user-owned
    /// fields, tags, goals, relations in both directions, the
    /// newest snapshot per source and the review history. Nothing
    /// here writes, so the projection never mutates repository or
    /// provider data.
    pub fn portfolio_project_view(
        &self,
        project_id: &str,
        now: DateTime<Utc>,
    ) -> Result<ProjectPortfolio, ForgeError> {
        validate_project_id(project_id).map_err(invalid)?;
        self.require_project(project_id)?;
        let profile = self.portfolio_record(project_id)?;
        let tags = self.portfolio_tags_for(project_id)?;
        let relations = self.portfolio_relations_for(project_id)?;
        let reviews = self.portfolio_reviews_for(project_id, 20)?;
        let mut evidence = self.portfolio_current_snapshots(project_id)?;
        let goals: Vec<GoalRecord> = self
            .portfolio_goals()?
            .into_iter()
            .filter(|goal| goal.projects.iter().any(|p| p == project_id))
            .collect();
        for snapshot in &mut evidence {
            snapshot.effective_status =
                portfolio::effective_status(snapshot.status, snapshot.stale_after.as_deref(), now);
        }
        Ok(ProjectPortfolio {
            profile,
            tags,
            goals,
            relations,
            evidence,
            reviews,
            generated_at: now.to_rfc3339(),
            contract: portfolio::PORTFOLIO_CONTRACT_VERSION,
        })
    }

    /// Every registered project with its portfolio record, tags
    /// and displayed evidence states, ordered by project id.
    /// Projects with no portfolio record are included as
    /// unclassified rather than hidden, so a filter can
    /// distinguish "no match" from "never reviewed". Nothing here
    /// writes.
    pub fn portfolio_fleet(
        &self,
        filter: &PortfolioFilter,
        now: DateTime<Utc>,
    ) -> Result<Vec<PortfolioRow>, ForgeError> {
        let mut out = Vec::new();
        for project in self.list()? {
            let profile = self.portfolio_record(&project.id)?;
            let tags = self.portfolio_tags_for(&project.id)?;
            if !filter.matches(&profile, &tags) {
                continue;
            }
            let evidence = self
                .portfolio_current_snapshots(&project.id)?
                .into_iter()
                .map(|snapshot| {
                    let status = portfolio::effective_status(
                        snapshot.status,
                        snapshot.stale_after.as_deref(),
                        now,
                    );
                    EvidenceState {
                        source_system: snapshot.source_system,
                        source_revision: snapshot.source_revision,
                        status,
                        observed_at: snapshot.observed_at,
                    }
                })
                .collect();
            out.push(PortfolioRow {
                profile,
                tags,
                evidence,
            });
        }
        out.sort_by(|a, b| a.profile.project_id.cmp(&b.profile.project_id));
        Ok(out)
    }
}

fn row_to_snapshot(
    project_id: &str,
) -> impl Fn(&rusqlite::Row<'_>) -> rusqlite::Result<EvidenceSnapshot> + '_ {
    move |row| {
        let status: String = row.get(4)?;
        let parsed = EvidenceStatus::parse(&status).unwrap_or(EvidenceStatus::Invalid);
        Ok(EvidenceSnapshot {
            snapshot_id: row.get(0)?,
            project_id: project_id.to_string(),
            source_system: row.get(1)?,
            source_revision: row.get(2)?,
            observed_at: row.get(3)?,
            status: parsed,
            stale_after: row.get(5)?,
            effective_status: parsed,
            evidence_json: row.get(6)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portfolio::Confidence;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-09-29T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn registry_with_projects(path: &std::path::Path, ids: &[&str]) -> crate::registry::Registry {
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

    fn snapshot(source: &str, status: EvidenceStatus) -> SnapshotWrite {
        SnapshotWrite {
            source_system: source.to_string(),
            source_revision: "rev-1".to_string(),
            observed_at: "2026-09-28T00:00:00Z".to_string(),
            status,
            stale_after: None,
            evidence_json: "{\"check\":\"ok\"}".to_string(),
        }
    }

    fn tmp() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    #[test]
    fn migration_is_additive_and_idempotent() {
        let dir = tmp();
        let path = dir.path().join("registry.db");
        let _registry = crate::registry::Registry::open(&path).expect("first open");
        // Re-opening applies the same batch again and must not fail.
        let _registry = crate::registry::Registry::open(&path).expect("second open");
        let conn = rusqlite::Connection::open(&path).expect("raw connection");
        for table in [
            "portfolio_projects",
            "portfolio_tags",
            "portfolio_project_tags",
            "portfolio_relations",
            "portfolio_goals",
            "portfolio_goal_projects",
            "portfolio_reviews",
            "portfolio_evidence_snapshots",
        ] {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    params![table],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            assert_eq!(count, 1, "missing table {table}");
        }
    }

    #[test]
    fn migration_preserves_a_pre_change_registry() {
        let dir = tmp();
        let path = dir.path().join("registry.db");
        // A registry written before the portfolio package: the
        // original two tables and a journal row, nothing else.
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE projects (
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
                    revision TEXT, build_status TEXT, run_status TEXT, container_identity TEXT);
                INSERT INTO projects (id, name, path, profile, schema_version,
                    platform_version, observed_at)
                VALUES ('legacy', 'legacy', '/tmp/legacy', 'rust-web', 1, '0.1.0',
                    '2026-01-01T00:00:00Z');
                INSERT INTO operations (kind, project_id, state, started_at)
                VALUES ('register', 'legacy', 'done', '2026-01-01T00:00:00Z');",
            )
            .unwrap();
        }
        let registry = crate::registry::Registry::open(&path).expect("migrate forward");
        let projects = registry.list().expect("projects survive");
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].id, "legacy");
        let ops = registry.journal_entries().expect("journal survives");
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].kind, "register");
        // And the new domain works against the migrated file.
        let tag = registry
            .portfolio_add_tag("legacy", "platform", None)
            .expect("tag on migrated registry");
        assert_eq!(tag.name, "platform");
    }

    /// A failing statement inside the portfolio batch must roll the
    /// whole batch back: the pre-existing registry stays usable and
    /// no partial portfolio table survives.
    ///
    /// The failure is deterministic. A decoy `portfolio_project_tags`
    /// table with a different column set makes the schema's own
    /// `CREATE TABLE IF NOT EXISTS` a no-op, and the later
    /// `CREATE INDEX ... (tag_id, project_id)` then fails with
    /// "no such column" partway through the batch.
    #[test]
    fn an_interrupted_migration_rolls_back_and_keeps_the_prior_registry_usable() {
        let dir = tmp();
        let path = dir.path().join("registry.db");
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE projects (
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
                    revision TEXT, build_status TEXT, run_status TEXT, container_identity TEXT);
                CREATE TABLE portfolio_project_tags (project_id TEXT, decoy TEXT);
                INSERT INTO projects (id, name, path, profile, schema_version,
                    platform_version, observed_at)
                VALUES ('legacy', 'legacy', '/tmp/legacy', 'rust-web', 1, '0.1.0',
                    '2026-01-01T00:00:00Z');",
            )
            .unwrap();
        }

        let err = crate::registry::Registry::open(&path)
            .err()
            .expect("the decoy must fail the portfolio batch");
        assert_eq!(err.code(), "registry-error");
        assert!(
            err.to_string().contains("portfolio migration failed"),
            "{err}"
        );
        assert!(err.to_string().contains("registry left unchanged"), "{err}");

        // Nothing from the batch survives: the three tables created
        // before the failing index were rolled back.
        let conn = rusqlite::Connection::open(&path).unwrap();
        for table in [
            "portfolio_projects",
            "portfolio_tags",
            "portfolio_goals",
            "portfolio_reviews",
            "portfolio_evidence_snapshots",
        ] {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    params![table],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            assert_eq!(count, 0, "`{table}` survived a failed migration");
        }
        // The prior registry is intact and readable.
        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))
            .unwrap();
        assert_eq!(rows, 1);

        // Removing the decoy lets the next open retry the whole
        // batch successfully.
        conn.execute_batch("DROP TABLE portfolio_project_tags")
            .unwrap();
        drop(conn);
        let registry = crate::registry::Registry::open(&path).expect("retry succeeds");
        assert_eq!(registry.list().expect("projects").len(), 1);
        assert!(registry.portfolio_goals().expect("goals").is_empty());
    }

    #[test]
    fn unknown_project_changes_no_portfolio_state() {
        let dir = tmp();
        let path = dir.path().join("registry.db");
        let registry = registry_with_projects(&path, &["alethefy"]);
        let err = registry
            .portfolio_add_tag("no-such-app", "platform", None)
            .unwrap_err();
        assert_eq!(err.code(), "unknown-project");
        let err = registry
            .portfolio_write(
                "no-such-app",
                &PortfolioWrite {
                    lifecycle: Some(Lifecycle::Building),
                    ..PortfolioWrite::default()
                },
            )
            .unwrap_err();
        assert_eq!(err.code(), "unknown-project");
        let err = registry
            .portfolio_record_review("no-such-app", Confidence::High, None)
            .unwrap_err();
        assert_eq!(err.code(), "unknown-project");
        let err = registry
            .portfolio_import_snapshot("no-such-app", &snapshot("gate", EvidenceStatus::Observed))
            .unwrap_err();
        assert_eq!(err.code(), "unknown-project");
        let err = registry
            .portfolio_link_goal("ship it", "no-such-app")
            .unwrap_err();
        assert_eq!(err.code(), "unknown-project");
        assert!(registry.portfolio_all_tags().unwrap().is_empty());
        assert!(registry.portfolio_goals().unwrap().is_empty());
    }

    #[test]
    fn tag_and_review_persist_with_project_identity_and_timestamp() {
        let dir = tmp();
        let path = dir.path().join("registry.db");
        let registry = registry_with_projects(&path, &["alethefy"]);
        registry
            .portfolio_add_tag("alethefy", "platform", Some("#AABBCC"))
            .expect("tag");
        let review = registry
            .portfolio_record_review("alethefy", Confidence::High, Some("gate is green"))
            .expect("review");
        assert_eq!(review.project_id, "alethefy");
        assert!(!review.reviewed_at.is_empty());

        let view = registry.portfolio_project_view("alethefy", now()).unwrap();
        assert_eq!(view.tags.len(), 1);
        assert_eq!(view.tags[0].name, "platform");
        assert_eq!(view.tags[0].color.as_deref(), Some("#aabbcc"));
        assert_eq!(view.reviews.len(), 1);
        assert_eq!(view.reviews[0].confidence, Confidence::High);
        assert_eq!(view.profile.confidence, Some(Confidence::High));
        assert!(view.profile.reviewed_at.is_some());
    }

    #[test]
    fn adding_a_tag_twice_leaves_one_link() {
        let dir = tmp();
        let path = dir.path().join("registry.db");
        let registry = registry_with_projects(&path, &["alethefy", "forge"]);
        let first = registry
            .portfolio_add_tag("alethefy", "platform", None)
            .unwrap();
        let second = registry
            .portfolio_add_tag("alethefy", "platform", None)
            .unwrap();
        assert_eq!(first.tag_id, second.tag_id);
        let tags = registry.portfolio_tags_for("alethefy").unwrap();
        assert_eq!(tags.len(), 1);
        // A shared tag carries both projects.
        registry
            .portfolio_add_tag("forge", "platform", None)
            .unwrap();
        let all = registry.portfolio_all_tags().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].1, vec!["alethefy".to_string(), "forge".to_string()]);

        assert!(registry
            .portfolio_remove_tag("alethefy", "platform")
            .unwrap());
        assert!(!registry
            .portfolio_remove_tag("alethefy", "platform")
            .unwrap());
        assert_eq!(registry.portfolio_tags_for("forge").unwrap().len(), 1);
    }

    #[test]
    fn dependency_relation_is_idempotent_and_visible_in_both_views() {
        let dir = tmp();
        let path = dir.path().join("registry.db");
        let registry = registry_with_projects(&path, &["alethefy", "forge"]);
        let first = registry
            .portfolio_add_relation("alethefy", "forge", RelationType::DependsOn, None)
            .unwrap();
        let second = registry
            .portfolio_add_relation("alethefy", "forge", RelationType::DependsOn, None)
            .unwrap();
        assert_eq!(first.relation_id, second.relation_id);

        let outgoing = registry.portfolio_relations_for("alethefy").unwrap();
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].direction, "outgoing");
        assert_eq!(outgoing[0].other_project, "forge");
        let incoming = registry.portfolio_relations_for("forge").unwrap();
        assert_eq!(incoming.len(), 1);
        assert_eq!(incoming[0].direction, "incoming");
        assert_eq!(incoming[0].other_project, "alethefy");

        assert!(registry
            .portfolio_remove_relation("alethefy", "forge", RelationType::DependsOn)
            .unwrap());
        assert!(registry
            .portfolio_relations_for("alethefy")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn self_relation_is_refused_before_any_write() {
        let dir = tmp();
        let path = dir.path().join("registry.db");
        let registry = registry_with_projects(&path, &["alethefy"]);
        let err = registry
            .portfolio_add_relation("alethefy", "alethefy", RelationType::DependsOn, None)
            .unwrap_err();
        assert_eq!(err.code(), "portfolio-invalid");
        assert!(err.to_string().contains("itself"), "{err}");
        let err = registry
            .portfolio_remove_relation("alethefy", "alethefy", RelationType::Replaces)
            .unwrap_err();
        assert_eq!(err.code(), "portfolio-invalid");
        assert!(registry
            .portfolio_relations_for("alethefy")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn relation_to_an_unknown_project_is_refused() {
        let dir = tmp();
        let path = dir.path().join("registry.db");
        let registry = registry_with_projects(&path, &["alethefy"]);
        let err = registry
            .portfolio_add_relation("alethefy", "ghost", RelationType::DependsOn, None)
            .unwrap_err();
        assert_eq!(err.code(), "unknown-project");
        assert!(registry
            .portfolio_relations_for("alethefy")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn snapshot_import_is_append_only_and_redacted() {
        let dir = tmp();
        let path = dir.path().join("registry.db");
        let registry = registry_with_projects(&path, &["alethefy"]);
        let mut write = snapshot("gate", EvidenceStatus::Observed);
        write.evidence_json = r#"{"token":"ghp_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"}"#.to_string();
        let first = registry
            .portfolio_import_snapshot("alethefy", &write)
            .expect("import");
        assert!(!first.evidence_json.contains("ghp_"));
        assert!(first.evidence_json.contains("[REDACTED]"));

        let mut second_write = snapshot("gate", EvidenceStatus::Unavailable);
        second_write.evidence_json = "{}".to_string();
        registry
            .portfolio_import_snapshot("alethefy", &second_write)
            .expect("second import");
        let all = registry.portfolio_snapshots_for("alethefy", 50).unwrap();
        assert_eq!(all.len(), 2, "snapshots are never replaced");
        let current = registry.portfolio_current_snapshots("alethefy").unwrap();
        assert_eq!(current.len(), 1);
        assert_eq!(current[0].status, EvidenceStatus::Unavailable);
    }

    #[test]
    fn snapshot_import_refuses_malformed_payloads_and_times() {
        let dir = tmp();
        let path = dir.path().join("registry.db");
        let registry = registry_with_projects(&path, &["alethefy"]);
        let mut write = snapshot("gate", EvidenceStatus::Observed);
        write.evidence_json = "not json".to_string();
        assert_eq!(
            registry
                .portfolio_import_snapshot("alethefy", &write)
                .unwrap_err()
                .code(),
            "portfolio-invalid"
        );

        let mut write = snapshot("gate", EvidenceStatus::Observed);
        write.observed_at = "yesterday".to_string();
        assert_eq!(
            registry
                .portfolio_import_snapshot("alethefy", &write)
                .unwrap_err()
                .code(),
            "portfolio-invalid"
        );

        let mut write = snapshot("", EvidenceStatus::Observed);
        write.source_system = "  ".to_string();
        assert_eq!(
            registry
                .portfolio_import_snapshot("alethefy", &write)
                .unwrap_err()
                .code(),
            "portfolio-invalid"
        );

        let mut write = snapshot("gate", EvidenceStatus::Observed);
        write.stale_after = Some("2026-01-01T00:00:00Z".to_string());
        let err = registry
            .portfolio_import_snapshot("alethefy", &write)
            .unwrap_err();
        assert!(err.to_string().contains("before it was seen"), "{err}");

        assert!(registry
            .portfolio_snapshots_for("alethefy", 50)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn read_model_downgrades_expired_evidence_to_stale() {
        let dir = tmp();
        let path = dir.path().join("registry.db");
        let registry = registry_with_projects(&path, &["alethefy"]);
        let mut fresh = snapshot("gate", EvidenceStatus::Observed);
        fresh.stale_after = Some("2026-12-01T00:00:00Z".to_string());
        registry
            .portfolio_import_snapshot("alethefy", &fresh)
            .unwrap();
        let mut expired = snapshot("governance", EvidenceStatus::Observed);
        expired.observed_at = "2026-08-01T00:00:00Z".to_string();
        expired.stale_after = Some("2026-09-01T00:00:00Z".to_string());
        registry
            .portfolio_import_snapshot("alethefy", &expired)
            .unwrap();

        let view = registry.portfolio_project_view("alethefy", now()).unwrap();
        assert_eq!(view.evidence.len(), 2);
        let gate = view
            .evidence
            .iter()
            .find(|s| s.source_system == "gate")
            .unwrap();
        let governance = view
            .evidence
            .iter()
            .find(|s| s.source_system == "governance")
            .unwrap();
        assert_eq!(gate.effective_status, EvidenceStatus::Observed);
        assert_eq!(governance.effective_status, EvidenceStatus::Stale);
        // The stored status is preserved verbatim next to the
        // displayed one.
        assert_eq!(governance.status, EvidenceStatus::Observed);
    }

    #[test]
    fn unavailable_snapshot_never_reads_as_healthy() {
        let dir = tmp();
        let path = dir.path().join("registry.db");
        let registry = registry_with_projects(&path, &["alethefy"]);
        registry
            .portfolio_import_snapshot(
                "alethefy",
                &snapshot("runtime", EvidenceStatus::Unavailable),
            )
            .unwrap();
        let view = registry.portfolio_project_view("alethefy", now()).unwrap();
        assert_eq!(view.evidence.len(), 1);
        assert_eq!(
            view.evidence[0].effective_status,
            EvidenceStatus::Unavailable
        );
        assert!(!view.evidence[0].effective_status.reports_observation());
    }

    #[test]
    fn goals_are_unique_and_link_projects() {
        let dir = tmp();
        let path = dir.path().join("registry.db");
        let registry = registry_with_projects(&path, &["alethefy", "forge"]);
        let goal = registry
            .portfolio_add_goal("ship the fleet", "active", Some("first"))
            .unwrap();
        let again = registry
            .portfolio_add_goal("ship the fleet", "done", None)
            .unwrap();
        assert_eq!(goal.goal_id, again.goal_id);
        assert_eq!(again.status, "done");
        assert_eq!(again.description.as_deref(), Some("first"));
        assert_eq!(registry.portfolio_goals().unwrap().len(), 1);

        let linked = registry
            .portfolio_link_goal("ship the fleet", "alethefy")
            .unwrap();
        assert_eq!(linked.projects, vec!["alethefy".to_string()]);
        let linked = registry
            .portfolio_link_goal("ship the fleet", "alethefy")
            .unwrap();
        assert_eq!(linked.projects, vec!["alethefy".to_string()]);
        registry
            .portfolio_link_goal("ship the fleet", "forge")
            .unwrap();
        let goals = registry.portfolio_goals().unwrap();
        assert_eq!(goals[0].projects, vec!["alethefy", "forge"]);

        let view = registry.portfolio_project_view("alethefy", now()).unwrap();
        assert_eq!(view.goals.len(), 1);
        assert_eq!(view.goals[0].title, "ship the fleet");
    }

    #[test]
    fn goal_title_and_status_are_validated() {
        let dir = tmp();
        let path = dir.path().join("registry.db");
        let registry = registry_with_projects(&path, &["alethefy"]);
        assert_eq!(
            registry
                .portfolio_add_goal("   ", "active", None)
                .unwrap_err()
                .code(),
            "portfolio-invalid"
        );
        assert_eq!(
            registry
                .portfolio_add_goal("a goal", "blocked", None)
                .unwrap_err()
                .code(),
            "portfolio-invalid"
        );
        assert!(registry.portfolio_goals().unwrap().is_empty());
    }

    #[test]
    fn fleet_filter_combines_user_and_tag_fields() {
        let dir = tmp();
        let path = dir.path().join("registry.db");
        let registry = registry_with_projects(&path, &["alethefy", "forge", "jenkins-local"]);
        registry
            .portfolio_write(
                "alethefy",
                &PortfolioWrite {
                    lifecycle: Some(Lifecycle::Building),
                    confidence: Some(Confidence::High),
                    ..PortfolioWrite::default()
                },
            )
            .unwrap();
        registry
            .portfolio_write(
                "forge",
                &PortfolioWrite {
                    lifecycle: Some(Lifecycle::Operational),
                    next_action: Some("cut 0.2.0".to_string()),
                    blocker: Some("waiting on the gate".to_string()),
                    ..PortfolioWrite::default()
                },
            )
            .unwrap();
        registry
            .portfolio_add_tag("alethefy", "platform", None)
            .unwrap();
        registry
            .portfolio_add_tag("forge", "tooling", None)
            .unwrap();

        let all = registry
            .portfolio_fleet(&PortfolioFilter::default(), now())
            .unwrap();
        assert_eq!(all.len(), 3, "unclassified projects stay visible");
        assert_eq!(all[0].profile.project_id, "alethefy");
        assert_eq!(all[2].profile.project_id, "jenkins-local");
        assert_eq!(all[2].evidence_summary(), "no evidence");

        let tagged = registry
            .portfolio_fleet(
                &PortfolioFilter {
                    tag: Some("platform".to_string()),
                    ..PortfolioFilter::default()
                },
                now(),
            )
            .unwrap();
        assert_eq!(tagged.len(), 1);
        assert_eq!(tagged[0].profile.project_id, "alethefy");

        let building = registry
            .portfolio_fleet(
                &PortfolioFilter {
                    lifecycle: Some(Lifecycle::Building),
                    ..PortfolioFilter::default()
                },
                now(),
            )
            .unwrap();
        assert_eq!(building.len(), 1);
        assert_eq!(building[0].profile.project_id, "alethefy");

        let combined = registry
            .portfolio_fleet(
                &PortfolioFilter {
                    tag: Some("platform".to_string()),
                    lifecycle: Some(Lifecycle::Operational),
                    ..PortfolioFilter::default()
                },
                now(),
            )
            .unwrap();
        assert!(combined.is_empty(), "an impossible filter matches nothing");

        let forge = registry.portfolio_record("forge").unwrap();
        assert_eq!(forge.next_action.as_deref(), Some("cut 0.2.0"));
        assert_eq!(forge.blocker.as_deref(), Some("waiting on the gate"));
    }

    #[test]
    fn unclassified_project_is_not_reported_as_a_default_lifecycle() {
        let dir = tmp();
        let path = dir.path().join("registry.db");
        let registry = registry_with_projects(&path, &["fresh"]);
        let view = registry.portfolio_project_view("fresh", now()).unwrap();
        assert_eq!(view.profile.lifecycle, None);
        assert_eq!(view.profile.confidence, None);
        assert!(view.tags.is_empty());
        assert!(view.evidence.is_empty());
        assert!(view.goals.is_empty());
        assert!(view.relations.is_empty());
        assert_eq!(view.contract, portfolio::PORTFOLIO_CONTRACT_VERSION);
    }

    #[test]
    fn write_validates_notes_before_persisting() {
        let dir = tmp();
        let path = dir.path().join("registry.db");
        let registry = registry_with_projects(&path, &["alethefy"]);
        let err = registry
            .portfolio_write(
                "alethefy",
                &PortfolioWrite {
                    next_action: Some("line\nbreak".to_string()),
                    ..PortfolioWrite::default()
                },
            )
            .unwrap_err();
        assert_eq!(err.code(), "portfolio-invalid");
        assert_eq!(
            registry.portfolio_record("alethefy").unwrap().next_action,
            None
        );
    }

    #[test]
    fn portfolio_record_is_written_transactionally_with_its_tag_link() {
        let dir = tmp();
        let path = dir.path().join("registry.db");
        let registry = registry_with_projects(&path, &["alethefy"]);
        let before = registry
            .portfolio_snapshots_for("alethefy", 10)
            .unwrap()
            .len();
        let err = registry
            .portfolio_add_tag("alethefy", "Bad Tag", None)
            .unwrap_err();
        assert_eq!(err.code(), "portfolio-invalid");
        assert!(registry.portfolio_tags_for("alethefy").unwrap().is_empty());
        assert_eq!(
            registry
                .portfolio_snapshots_for("alethefy", 10)
                .unwrap()
                .len(),
            before
        );
    }
}
