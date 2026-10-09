//! Portfolio registry methods.

use crate::core::{validate_project_id, ForgeError};
use crate::portfolio::{
    self, Confidence, EvidenceSnapshot, EvidenceState, GoalRecord, Lifecycle, PortfolioFilter,
    PortfolioProject, PortfolioRow, ProjectPortfolio, ProjectRelation, RelationRecord,
    RelationType, ReviewRecord, TagRecord,
};
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};

use super::snapshots::row_to_snapshot;
use super::validate::{invalid, validate_snapshot};
use super::writes::{PortfolioWrite, SnapshotWrite};

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
