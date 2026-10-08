//! CLI surface: migration, help/shape, metadata, relations, goals, evidence, compatibility.

use super::*;

// --- migration ----------------------------------------------------------

/// A registry written before the portfolio package: the original
/// `projects`/`operations` tables and one row in each, nothing
/// else. Opening it must migrate forward additively and leave
/// both rows usable.
#[test]
fn pre_change_registry_migrates_forward_without_losing_rows() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    {
        let conn = rusqlite::Connection::open(&db).unwrap();
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

    // A plain `forge list` opens the registry and therefore runs
    // the migration.
    let listed = run_json(&db, &["list"]);
    assert_eq!(
        listed["projects"][0]["id"], "legacy",
        "the pre-change project row must survive the migration"
    );

    // The journal is preserved too.
    let status = run_json(&db, &["deploy", "status"]);
    assert_eq!(status["contract"], "forge-deploy-status/0.2.0");

    // And the portfolio domain works against the migrated file.
    let tagged = run_json(
        &db,
        &["portfolio", "tag", "add", "legacy", "--name", "platform"],
    );
    assert_eq!(tagged["tag"]["name"], "platform");
    let tags = run_json(&db, &["portfolio", "tag", "list", "legacy"]);
    assert_eq!(tags["tags"].as_array().unwrap().len(), 1);
}

#[test]
fn opening_twice_keeps_the_portfolio_schema_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    run(&db, &["list"]);
    run(&db, &["portfolio", "goal", "list"]);
    run(&db, &["list"]);
    let conn = rusqlite::Connection::open(&db).unwrap();
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
                rusqlite::params![table],
                |row| row.get(0),
            )
            .unwrap_or(0);
        assert_eq!(count, 1, "missing portfolio table {table}");
    }
}

// --- CLI help and shape -------------------------------------------------

#[test]
fn help_advertises_every_portfolio_operation() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["portfolio", "--help"]);
    let stdout = lossy(&out.stdout);
    for expected in ["tag", "relation", "review", "goal", "evidence", "show"] {
        assert!(stdout.contains(expected), "missing `{expected}`: {stdout}");
    }
    let out = run(&db, &["portfolio", "tag", "--help"]);
    assert!(lossy(&out.stdout).contains("remove"));
    let out = run(&db, &["portfolio", "evidence", "import", "--help"]);
    let stdout = lossy(&out.stdout);
    for expected in [
        "--source",
        "--revision",
        "--status",
        "--observed-at",
        "--stale-after",
        "--evidence",
    ] {
        assert!(stdout.contains(expected), "missing `{expected}`: {stdout}");
    }
}

// --- user-owned metadata ------------------------------------------------

#[test]
fn tag_and_review_persist_with_project_identity_and_timestamp() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _) = registered(tmp.path(), "alethefy");

    let tagged = run_json(
        &db,
        &[
            "portfolio",
            "tag",
            "add",
            "alethefy",
            "--name",
            "platform",
            "--color",
            "#aabbcc",
        ],
    );
    assert_eq!(tagged["tag"]["name"], "platform");
    assert_eq!(tagged["tag"]["color"], "#aabbcc");

    let reviewed = run_json(
        &db,
        &[
            "portfolio",
            "review",
            "set",
            "alethefy",
            "--confidence",
            "high",
            "--note",
            "gate is green",
            "--lifecycle",
            "building",
            "--next-action",
            "cut 0.2.0",
            "--blocker",
            "waiting on the mac",
        ],
    );
    assert_eq!(reviewed["review"]["project_id"], "alethefy");
    assert!(!reviewed["review"]["reviewed_at"]
        .as_str()
        .unwrap()
        .is_empty());
    assert_eq!(reviewed["profile"]["lifecycle"], "building");
    assert_eq!(reviewed["profile"]["confidence"], "high");

    let shown = run_json(&db, &["portfolio", "show", "alethefy"]);
    assert_eq!(shown["profile"]["lifecycle"], "building");
    assert_eq!(shown["profile"]["next_action"], "cut 0.2.0");
    assert_eq!(shown["profile"]["blocker"], "waiting on the mac");
    let tags = shown["tags"].as_array().unwrap();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0]["name"], "platform");
}

#[test]
fn unknown_project_is_a_typed_not_found_that_changes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _) = registered(tmp.path(), "alethefy");

    for args in [
        vec!["portfolio", "tag", "add", "ghost", "--name", "platform"],
        vec![
            "portfolio",
            "review",
            "set",
            "ghost",
            "--confidence",
            "high",
        ],
        vec![
            "portfolio",
            "evidence",
            "import",
            "--project",
            "ghost",
            "--source",
            "gate",
            "--revision",
            "r1",
            "--status",
            "observed",
        ],
        vec!["portfolio", "goal", "link", "ship it", "--project", "ghost"],
    ] {
        let out = run(&db, &args);
        assert!(!out.status.success(), "{args:?} must fail");
        let stderr = lossy(&out.stderr);
        assert!(
            stderr.contains("error[unknown-project]"),
            "{args:?}: {stderr}"
        );
        assert!(out.stdout.is_empty(), "{args:?} wrote stdout");
    }
    assert!(run_json(&db, &["portfolio", "tag", "list"])["tags"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(run_json(&db, &["portfolio", "goal", "list"])["goals"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn duplicate_tag_leaves_one_link_and_repeated_review_keeps_history() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _) = registered(tmp.path(), "alethefy");
    run(
        &db,
        &["portfolio", "tag", "add", "alethefy", "--name", "platform"],
    );
    run(
        &db,
        &["portfolio", "tag", "add", "alethefy", "--name", "platform"],
    );
    let tags = run_json(&db, &["portfolio", "tag", "list", "alethefy"]);
    assert_eq!(tags["tags"].as_array().unwrap().len(), 1);

    run(
        &db,
        &[
            "portfolio",
            "review",
            "set",
            "alethefy",
            "--confidence",
            "low",
        ],
    );
    run(
        &db,
        &[
            "portfolio",
            "review",
            "set",
            "alethefy",
            "--confidence",
            "high",
        ],
    );
    let reviews = run_json(&db, &["portfolio", "review", "list", "alethefy"]);
    assert_eq!(reviews["reviews"].as_array().unwrap().len(), 2);
    // The newest review is the current confidence statement.
    assert_eq!(reviews["reviews"][0]["confidence"], "high");
    let shown = run_json(&db, &["portfolio", "show", "alethefy"]);
    assert_eq!(shown["profile"]["confidence"], "high");
}

#[test]
fn invalid_tag_name_and_unknown_vocabulary_are_typed_refusals() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _) = registered(tmp.path(), "alethefy");
    for (args, expected) in [
        (
            vec!["portfolio", "tag", "add", "alethefy", "--name", "Two Words"],
            "portfolio-invalid",
        ),
        (
            vec!["portfolio", "tag", "add", "alethefy", "--name", ""],
            "portfolio-invalid",
        ),
        (
            vec![
                "portfolio",
                "review",
                "set",
                "alethefy",
                "--confidence",
                "certain",
            ],
            "portfolio-invalid",
        ),
        (
            vec![
                "portfolio",
                "review",
                "set",
                "alethefy",
                "--confidence",
                "high",
                "--lifecycle",
                "shipped",
            ],
            "portfolio-invalid",
        ),
        (
            vec![
                "portfolio",
                "relation",
                "add",
                "alethefy",
                "--to",
                "alethefy",
                "--type",
                "depends-on",
            ],
            "portfolio-invalid",
        ),
        (
            vec![
                "portfolio",
                "relation",
                "add",
                "alethefy",
                "--to",
                "alethefy",
                "--type",
                "blocks",
            ],
            "portfolio-invalid",
        ),
    ] {
        let out = run(&db, &args);
        assert!(!out.status.success(), "{args:?} must fail");
        assert!(
            lossy(&out.stderr).contains(expected),
            "{args:?}: {}",
            lossy(&out.stderr)
        );
    }
    assert!(run_json(&db, &["portfolio", "tag", "list"])["tags"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(
        run_json(&db, &["portfolio", "relation", "list"])["relations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

// --- relations ----------------------------------------------------------

#[test]
fn dependency_relation_is_idempotent_and_shown_in_both_project_views() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _) = registered(tmp.path(), "alethefy");
    registered(tmp.path(), "forge");

    for _ in 0..2 {
        let added = run_json(
            &db,
            &[
                "portfolio",
                "relation",
                "add",
                "alethefy",
                "--to",
                "forge",
                "--type",
                "depends-on",
                "--note",
                "shares the registry",
            ],
        );
        assert_eq!(added["relation"]["relation_type"], "depends-on");
    }
    let all = run_json(&db, &["portfolio", "relation", "list"]);
    assert_eq!(all["relations"].as_array().unwrap().len(), 1);

    let outgoing = run_json(&db, &["portfolio", "relation", "list", "alethefy"]);
    assert_eq!(outgoing["relations"][0]["direction"], "outgoing");
    assert_eq!(outgoing["relations"][0]["other_project"], "forge");
    let incoming = run_json(&db, &["portfolio", "relation", "list", "forge"]);
    assert_eq!(incoming["relations"][0]["direction"], "incoming");
    assert_eq!(incoming["relations"][0]["other_project"], "alethefy");

    // The relation appears in both projects' portfolio projections.
    for project in ["alethefy", "forge"] {
        let shown = run_json(&db, &["portfolio", "show", project]);
        let relations = shown["relations"].as_array().unwrap();
        assert_eq!(relations.len(), 1, "{project}");
    }

    run(
        &db,
        &[
            "portfolio",
            "relation",
            "remove",
            "alethefy",
            "--to",
            "forge",
            "--type",
            "depends-on",
        ],
    );
    assert!(
        run_json(&db, &["portfolio", "relation", "list"])["relations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

// --- goals --------------------------------------------------------------

#[test]
fn goals_are_unique_and_link_projects() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _) = registered(tmp.path(), "alethefy");
    run(
        &db,
        &[
            "portfolio",
            "goal",
            "add",
            "ship the fleet",
            "--status",
            "active",
        ],
    );
    let again = run_json(
        &db,
        &[
            "portfolio",
            "goal",
            "add",
            "ship the fleet",
            "--status",
            "done",
        ],
    );
    assert_eq!(again["goal"]["status"], "done");
    let linked = run_json(
        &db,
        &[
            "portfolio",
            "goal",
            "link",
            "ship the fleet",
            "--project",
            "alethefy",
        ],
    );
    assert_eq!(linked["goal"]["projects"][0], "alethefy");
    let goals = run_json(&db, &["portfolio", "goal", "list"]);
    assert_eq!(goals["goals"].as_array().unwrap().len(), 1);
}

// --- evidence snapshots -------------------------------------------------

#[test]
fn fresh_evidence_is_stored_with_its_source_and_revision() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _) = registered(tmp.path(), "alethefy");
    let imported = run_json(
        &db,
        &[
            "portfolio",
            "evidence",
            "import",
            "--project",
            "alethefy",
            "--source",
            "workspace-governance",
            "--revision",
            "a1b2c3",
            "--status",
            "observed",
            "--observed-at",
            "2026-09-28T00:00:00Z",
            "--evidence",
            r#"{"independent":true}"#,
        ],
    );
    assert_eq!(
        imported["snapshot"]["source_system"],
        "workspace-governance"
    );
    assert_eq!(imported["snapshot"]["source_revision"], "a1b2c3");
    assert_eq!(imported["snapshot"]["status"], "observed");

    let shown = run_json(&db, &["portfolio", "show", "alethefy"]);
    assert_eq!(
        shown["evidence"][0]["source_system"],
        "workspace-governance"
    );
    assert_eq!(shown["evidence"][0]["source_revision"], "a1b2c3");
    assert_eq!(shown["evidence"][0]["effective_status"], "observed");
}

#[test]
fn expired_evidence_reads_stale_and_unavailable_never_reads_healthy() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _) = registered(tmp.path(), "alethefy");
    run(
        &db,
        &[
            "portfolio",
            "evidence",
            "import",
            "--project",
            "alethefy",
            "--source",
            "governance",
            "--revision",
            "old",
            "--status",
            "observed",
            "--observed-at",
            "2026-01-01T00:00:00Z",
            "--stale-after",
            "2026-02-01T00:00:00Z",
        ],
    );
    run(
        &db,
        &[
            "portfolio",
            "evidence",
            "import",
            "--project",
            "alethefy",
            "--source",
            "runtime",
            "--revision",
            "none",
            "--status",
            "unavailable",
        ],
    );

    let listed = run_json(&db, &["portfolio", "evidence", "list", "alethefy"]);
    let snapshots = listed["snapshots"].as_array().unwrap();
    assert_eq!(snapshots.len(), 2);
    let governance = snapshots
        .iter()
        .find(|s| s["source_system"] == "governance")
        .unwrap();
    assert_eq!(governance["effective_status"], "stale");
    assert_eq!(governance["status"], "observed");
    let runtime = snapshots
        .iter()
        .find(|s| s["source_system"] == "runtime")
        .unwrap();
    assert_eq!(runtime["effective_status"], "unavailable");

    let human = lossy(&run(&db, &["portfolio", "evidence", "list", "alethefy"]).stdout);
    assert!(human.contains("governance observed stale"), "{human}");
    assert!(human.contains("runtime unavailable unavailable"), "{human}");
}

#[test]
fn evidence_import_refuses_malformed_payload_and_time() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _) = registered(tmp.path(), "alethefy");
    let base = [
        "portfolio",
        "evidence",
        "import",
        "--project",
        "alethefy",
        "--source",
        "gate",
        "--revision",
        "r1",
        "--status",
        "observed",
    ];
    let mut args = base.to_vec();
    args.extend_from_slice(&["--evidence", "not json"]);
    let out = run(&db, &args);
    assert!(!out.status.success());
    assert!(
        lossy(&out.stderr).contains("portfolio-invalid"),
        "{}",
        lossy(&out.stderr)
    );

    let mut args = base.to_vec();
    args.extend_from_slice(&["--observed-at", "yesterday"]);
    assert!(!run(&db, &args).status.success());

    let mut args = base.to_vec();
    args.extend_from_slice(&["--stale-after", "2020-01-01T00:00:00Z"]);
    let out = run(&db, &args);
    assert!(!out.status.success());
    assert!(
        lossy(&out.stderr).contains("before it was seen"),
        "{}",
        lossy(&out.stderr)
    );

    assert!(
        run_json(&db, &["portfolio", "evidence", "list", "alethefy"])["snapshots"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn evidence_payload_is_redacted_before_it_is_stored() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _) = registered(tmp.path(), "alethefy");
    let imported = run_json(
        &db,
        &[
            "portfolio",
            "evidence",
            "import",
            "--project",
            "alethefy",
            "--source",
            "gate",
            "--revision",
            "r1",
            "--status",
            "observed",
            "--evidence",
            r#"{"token":"ghp_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"}"#,
        ],
    );
    let payload = imported["snapshot"]["evidence_json"].as_str().unwrap();
    assert!(!payload.contains("ghp_"), "{payload}");
    assert!(payload.contains("[REDACTED]"), "{payload}");

    // The raw database never holds the secret either.
    let conn = rusqlite::Connection::open(&db).unwrap();
    let stored: String = conn
        .query_row(
            "SELECT evidence_json FROM portfolio_evidence_snapshots LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!stored.contains("ghp_"), "{stored}");
}

#[test]
fn evidence_import_can_read_a_fixture_file_and_bounds_it() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _) = registered(tmp.path(), "alethefy");
    let fixture = tmp.path().join("gate-report.json");
    fs::write(&fixture, r#"{"verdict":"passed","revision":"a1b2c3"}"#).unwrap();
    let imported = run_json(
        &db,
        &[
            "portfolio",
            "evidence",
            "import",
            "--project",
            "alethefy",
            "--source",
            "driftwatchdog",
            "--revision",
            "a1b2c3",
            "--status",
            "observed",
            "--evidence",
            &format!("@{}", fixture.display()),
        ],
    );
    assert!(imported["snapshot"]["evidence_json"]
        .as_str()
        .unwrap()
        .contains("a1b2c3"));

    let missing = tmp.path().join("absent.json");
    let out = run(
        &db,
        &[
            "portfolio",
            "evidence",
            "import",
            "--project",
            "alethefy",
            "--source",
            "driftwatchdog",
            "--revision",
            "a1b2c3",
            "--status",
            "observed",
            "--evidence",
            &format!("@{}", missing.display()),
        ],
    );
    assert!(!out.status.success());
    assert!(
        lossy(&out.stderr).contains("portfolio-invalid"),
        "{}",
        lossy(&out.stderr)
    );
}

#[test]
fn snapshots_are_append_only_and_never_rewritten() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _) = registered(tmp.path(), "alethefy");
    for (revision, status) in [("r1", "observed"), ("r2", "unavailable")] {
        run(
            &db,
            &[
                "portfolio",
                "evidence",
                "import",
                "--project",
                "alethefy",
                "--source",
                "gate",
                "--revision",
                revision,
                "--status",
                status,
            ],
        );
    }
    // Both rows survive: the read model picks the newest per
    // source instead of overwriting history.
    let listed = run_json(&db, &["portfolio", "evidence", "list", "alethefy"]);
    assert_eq!(listed["snapshots"].as_array().unwrap().len(), 2);
    let shown = run_json(&db, &["portfolio", "show", "alethefy"]);
    assert_eq!(
        shown["evidence"].as_array().unwrap().len(),
        1,
        "the read model shows only the newest snapshot per source"
    );
    assert_eq!(shown["evidence"][0]["source_revision"], "r2");
    assert_eq!(shown["evidence"][0]["effective_status"], "unavailable");
}

#[test]
fn user_metadata_is_independent_of_external_state() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _) = registered(tmp.path(), "alethefy");
    run(
        &db,
        &[
            "portfolio",
            "review",
            "set",
            "alethefy",
            "--confidence",
            "high",
            "--lifecycle",
            "operational",
        ],
    );
    run(
        &db,
        &[
            "portfolio",
            "evidence",
            "import",
            "--project",
            "alethefy",
            "--source",
            "gate",
            "--revision",
            "r1",
            "--status",
            "unavailable",
        ],
    );
    let shown = run_json(&db, &["portfolio", "show", "alethefy"]);
    assert_eq!(
        shown["profile"]["lifecycle"], "operational",
        "an unavailable provider must not change the user's own classification"
    );
    assert_eq!(shown["profile"]["confidence"], "high");
    assert_eq!(shown["evidence"][0]["effective_status"], "unavailable");
}

// --- compatibility ------------------------------------------------------

/// The pre-existing registry, inventory, provider and CLI surfaces
/// keep answering after the portfolio migration ran: this change
/// adds tables, never columns on the shared tables.
#[test]
fn existing_registry_contracts_stay_compatible() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, dir) = registered(tmp.path(), "alethefy");
    run(
        &db,
        &["portfolio", "tag", "add", "alethefy", "--name", "platform"],
    );

    let listed = run_json(&db, &["list"]);
    assert_eq!(listed["projects"][0]["id"], "alethefy");
    // The project row shape is untouched by the portfolio tables.
    let record = listed["projects"][0].as_object().unwrap();
    for column in ["id", "name", "path", "profile", "maturity", "observed_at"] {
        assert!(record.contains_key(column), "missing `{column}`");
    }
    assert!(
        !record.contains_key("lifecycle"),
        "portfolio fields must not leak into the canonical project record"
    );

    let inspected = run_json(&db, &["inspect", dir.to_str().unwrap()]);
    assert_eq!(inspected["id"], "alethefy");
    let doctor = run_json(&db, &["doctor", dir.to_str().unwrap()]);
    assert!(doctor["doctor"]["findings"].is_array());
}
