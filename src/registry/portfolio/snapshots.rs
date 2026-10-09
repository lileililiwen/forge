//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::portfolio::{EvidenceSnapshot, EvidenceStatus};

pub(super) fn row_to_snapshot(
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
pub(super) mod tests {
    use crate::portfolio::{
        self, Confidence, EvidenceStatus, Lifecycle, PortfolioFilter, RelationType,
    };
    use crate::registry::portfolio::writes::{PortfolioWrite, SnapshotWrite};
    use chrono::{DateTime, Utc};
    use rusqlite::params;

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
