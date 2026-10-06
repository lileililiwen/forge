//! Portfolio metadata and evidence-snapshot contract
//! (`portfolio-metadata-and-review`).
//!
//! Covers the verification oracle in the change design:
//!
//! - Migration round trip: a registry written before the
//!   portfolio package migrates forward with its project and
//!   journal rows intact, and every new registry command works
//!   against the migrated file.
//! - Domain boundaries: unknown project, duplicate tag,
//!   invalid relation type, self relation, malformed evidence
//!   payload and expired freshness bound are all typed
//!   refusals that leave no portfolio state behind.
//! - Stale/unavailable projection: an expired freshness bound
//!   reads `stale` and an unavailable provider reads
//!   `unavailable`; neither is ever reported as a pass.
//! - API contract: the portfolio read and mutation routes answer
//!   through the existing authorization boundary, so an
//!   unauthorized mutation persists no change.
//!
//! The evidence import path is exercised with local fixtures
//! only; no external provider is contacted and no remote or
//! multi-user readiness is claimed here.

use std::collections::BTreeMap;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use chrono::Utc;
use forge::api::{handle_buffered, ApiConfig, ApiRequest, ApiResponse};
use forge::registry::Registry;
use serde_json::Value;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn clean_cmd() -> std::process::Command {
    let mut cmd = std::process::Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd
}

fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn run_json(db: &Path, args: &[&str]) -> Value {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("run forge json");
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; stdout={} stderr={}",
            lossy(&out.stdout),
            lossy(&out.stderr)
        )
    })
}

const IDENTITY_YAML: &str = "  provider: okta\n  issuer: https://example.okta.com\n  client_id: forge-admin\n  audience: forge-admin\n  redirect_uri: https://admin.example.com/oidc/callback\n  scopes:\n    - openid\n    - profile\n  admin_claim: groups\n  admin_values:\n    - forge-admins\n  state_ttl_seconds: 120\n  session_ttl_seconds: 3600\n  client_secret_ref: env://OIDC_CLIENT_SECRET\n";

fn write_identity_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\n  target_maturity: L2\nruntime:\n  language: rust\nidentity:\n{IDENTITY_YAML}"
    );
    fs::write(dir.join("forge.yaml"), text).unwrap();
    fs::create_dir_all(dir.join(".forge/identity")).unwrap();
    fs::write(
        dir.join(".forge/identity/config.yaml"),
        format!("schema: 1\nproject: {id}\nidentity:\n{IDENTITY_YAML}"),
    )
    .unwrap();
}

/// Register one identity-capable project and return `(db, dir)`.
fn registered(tmp: &Path, id: &str) -> (PathBuf, PathBuf) {
    let dir = tmp.join(id);
    write_identity_project(&dir, id);
    let db = tmp.join("registry.db");
    let out = run(&db, &["register", dir.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "register {id} failed: {}",
        lossy(&out.stderr)
    );
    (db, dir)
}

fn mint_session_token(db: &Path, project_dir: &Path) -> String {
    let challenge = run_json(
        db,
        &["identity", "build-challenge", project_dir.to_str().unwrap()],
    );
    let state = challenge["challenge"]["state"].as_str().unwrap();
    let nonce = challenge["challenge"]["nonce"].as_str().unwrap();
    let mint = run_json(
        db,
        &[
            "identity",
            "complete-auth",
            project_dir.to_str().unwrap(),
            "--state",
            state,
            "--code",
            "abcd1234",
            "--subject",
            "user-1",
            "--nonce",
            nonce,
            "--admin-claim-value",
            "forge-admins",
        ],
    );
    mint["outcome"]["Session"]["session_id"]
        .as_str()
        .expect("session id")
        .to_string()
}

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

// --- API contract -------------------------------------------------------

fn drive(config: &ApiConfig, db_path: &Path, request: &ApiRequest) -> ApiResponse {
    let mut full: Vec<u8> = Vec::new();
    full.extend_from_slice(request.method.as_bytes());
    full.extend_from_slice(b" ");
    full.extend_from_slice(request.path.as_bytes());
    full.extend_from_slice(b" HTTP/1.1\r\n");
    for (k, v) in &request.headers {
        full.extend_from_slice(k.as_bytes());
        full.extend_from_slice(b": ");
        full.extend_from_slice(v.as_bytes());
        full.extend_from_slice(b"\r\n");
    }
    if !request.body.is_empty() {
        full.extend_from_slice(format!("content-length: {}\r\n", request.body.len()).as_bytes());
    }
    full.extend_from_slice(b"\r\n");
    full.extend_from_slice(&request.body);
    let mut reader = Cursor::new(full);
    let mut writer = Vec::new();
    handle_buffered(config, db_path, &mut reader, &mut writer, 1024 * 1024)
        .expect("handle_buffered")
}

fn api_request(method: &str, path: &str, bearer: Option<&str>, body: &str) -> ApiRequest {
    let mut headers = BTreeMap::new();
    headers.insert("content-type".to_string(), "application/json".to_string());
    if let Some(token) = bearer {
        headers.insert("authorization".to_string(), format!("Bearer {token}"));
    }
    ApiRequest {
        method: method.to_string(),
        path: path.to_string(),
        query: None,
        headers,
        body: body.as_bytes().to_vec(),
        idempotency_key: None,
        bearer_token: bearer.map(|v| v.to_string()),
        cookies: BTreeMap::new(),
        remote_addr: Some("127.0.0.1:9999".parse().unwrap()),
        started_at: Utc::now(),
    }
}

fn api_json(response: &ApiResponse) -> Value {
    serde_json::from_slice(&response.body).unwrap_or(Value::Null)
}

fn seed_two_projects(tmp: &Path) -> (PathBuf, String) {
    let (db, dir) = registered(tmp, "alethefy");
    registered(tmp, "forge");
    let session = mint_session_token(&db, &dir);
    (db, session)
}

#[test]
fn api_portfolio_round_trip_answers_through_the_authorization_boundary() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, session) = seed_two_projects(tmp.path());
    let config = ApiConfig::default();

    // Read.
    let response = drive(
        &config,
        &db,
        &api_request("GET", "/v1/projects/alethefy/portfolio", Some(&session), ""),
    );
    assert_eq!(response.status, 200);
    let body = api_json(&response);
    assert_eq!(body["portfolio"]["profile"]["lifecycle"], Value::Null);
    assert_eq!(body["contract"], forge::api::API_CONTRACT_VERSION);

    // Tag.
    let response = drive(
        &config,
        &db,
        &api_request(
            "POST",
            "/v1/projects/alethefy/portfolio/tags",
            Some(&session),
            r#"{"name":"platform"}"#,
        ),
    );
    assert_eq!(
        response.status,
        200,
        "{}",
        String::from_utf8_lossy(&response.body)
    );
    assert_eq!(api_json(&response)["portfolio"]["tag"]["name"], "platform");

    // Relation.
    let response = drive(
        &config,
        &db,
        &api_request(
            "POST",
            "/v1/projects/alethefy/portfolio/relations",
            Some(&session),
            r#"{"to":"forge","type":"depends-on"}"#,
        ),
    );
    assert_eq!(response.status, 200);
    assert_eq!(
        api_json(&response)["portfolio"]["relation"]["relation_type"],
        "depends-on"
    );

    // Review.
    let response = drive(
        &config,
        &db,
        &api_request(
            "POST",
            "/v1/projects/alethefy/portfolio/reviews",
            Some(&session),
            r#"{"confidence":"high","lifecycle":"building","next_action":"cut 0.2.0"}"#,
        ),
    );
    assert_eq!(response.status, 200);
    assert_eq!(
        api_json(&response)["portfolio"]["review"]["confidence"],
        "high"
    );

    // Evidence.
    let response = drive(
        &config,
        &db,
        &api_request(
            "POST",
            "/v1/projects/alethefy/portfolio/evidence",
            Some(&session),
            r#"{"source":"gate","revision":"a1b2c3","status":"observed","evidence":{"verdict":"passed"}}"#,
        ),
    );
    assert_eq!(response.status, 200);
    assert_eq!(
        api_json(&response)["portfolio"]["snapshot"]["status"],
        "observed"
    );

    // The read projection carries everything back.
    let response = drive(
        &config,
        &db,
        &api_request("GET", "/v1/projects/alethefy/portfolio", Some(&session), ""),
    );
    let body = api_json(&response);
    let portfolio = &body["portfolio"];
    assert_eq!(portfolio["profile"]["lifecycle"], "building");
    assert_eq!(portfolio["tags"][0]["name"], "platform");
    assert_eq!(portfolio["relations"][0]["other_project"], "forge");
    assert_eq!(portfolio["evidence"][0]["source_revision"], "a1b2c3");
    assert_eq!(portfolio["reviews"][0]["confidence"], "high");
}

#[test]
fn api_portfolio_mutation_without_a_session_persists_no_change() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _) = seed_two_projects(tmp.path());
    let config = ApiConfig::default();

    for (path, body) in [
        (
            "/v1/projects/alethefy/portfolio/tags",
            r#"{"name":"platform"}"#,
        ),
        (
            "/v1/projects/alethefy/portfolio/reviews",
            r#"{"confidence":"high"}"#,
        ),
        (
            "/v1/projects/alethefy/portfolio/relations",
            r#"{"to":"forge","type":"depends-on"}"#,
        ),
        (
            "/v1/projects/alethefy/portfolio/evidence",
            r#"{"source":"gate","revision":"r1","status":"observed"}"#,
        ),
    ] {
        let response = drive(&config, &db, &api_request("POST", path, None, body));
        assert_eq!(response.status, 401, "{path}");
        assert_eq!(api_json(&response)["error"]["code"], "api-unauthorized");
    }

    // Nothing landed: the registry carries no portfolio rows.
    let tags = run_json(&db, &["portfolio", "tag", "list"]);
    assert!(tags["tags"].as_array().unwrap().is_empty());
    let relations = run_json(&db, &["portfolio", "relation", "list"]);
    assert!(relations["relations"].as_array().unwrap().is_empty());
    let reviews = run_json(&db, &["portfolio", "review", "list", "alethefy"]);
    assert!(reviews["reviews"].as_array().unwrap().is_empty());
    let evidence = run_json(&db, &["portfolio", "evidence", "list", "alethefy"]);
    assert!(evidence["snapshots"].as_array().unwrap().is_empty());
}

#[test]
fn api_portfolio_session_for_another_project_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _) = registered(tmp.path(), "alethefy");
    let forge_dir = tmp.path().join("forge");
    write_identity_project(&forge_dir, "forge");
    run(&db, &["register", forge_dir.to_str().unwrap()]);
    // A session minted for `forge`, presented to `alethefy`.
    let session = mint_session_token(&db, &forge_dir);
    let config = ApiConfig::default();
    let response = drive(
        &config,
        &db,
        &api_request(
            "POST",
            "/v1/projects/alethefy/portfolio/tags",
            Some(&session),
            r#"{"name":"platform"}"#,
        ),
    );
    assert_eq!(response.status, 403);
    assert_eq!(api_json(&response)["error"]["code"], "api-project-mismatch");
    assert!(run_json(&db, &["portfolio", "tag", "list"])["tags"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn api_portfolio_malformed_payloads_are_typed_bad_requests() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, session) = seed_two_projects(tmp.path());
    let config = ApiConfig::default();
    for (path, body, expected) in [
        ("/v1/projects/alethefy/portfolio/tags", "{}", "api-invalid"),
        (
            "/v1/projects/alethefy/portfolio/tags",
            r#"{"name":"Two Words"}"#,
            "portfolio-invalid",
        ),
        (
            "/v1/projects/alethefy/portfolio/reviews",
            r#"{"confidence":"certain"}"#,
            "api-invalid",
        ),
        (
            "/v1/projects/alethefy/portfolio/relations",
            r#"{"to":"forge","type":"blocks"}"#,
            "api-invalid",
        ),
        (
            "/v1/projects/alethefy/portfolio/relations",
            r#"{"to":"alethefy","type":"depends-on"}"#,
            "portfolio-invalid",
        ),
        (
            "/v1/projects/alethefy/portfolio/evidence",
            r#"{"source":"gate","revision":"r1","status":"observed","evidence":"nope"}"#,
            "portfolio-invalid",
        ),
        (
            "/v1/projects/alethefy/portfolio/evidence",
            r#"{"source":"Gate Provider","revision":"r1","status":"observed"}"#,
            "portfolio-invalid",
        ),
    ] {
        let response = drive(
            &config,
            &db,
            &api_request("POST", path, Some(&session), body),
        );
        assert_eq!(response.status, 400, "{path} {body}");
        assert_eq!(
            api_json(&response)["error"]["code"],
            expected,
            "{path} {body}"
        );
    }
    assert!(run_json(&db, &["portfolio", "tag", "list"])["tags"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn api_portfolio_read_of_an_unknown_project_is_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, session) = seed_two_projects(tmp.path());
    let config = ApiConfig::default();
    let response = drive(
        &config,
        &db,
        &api_request("GET", "/v1/projects/ghost/portfolio", Some(&session), ""),
    );
    assert_eq!(response.status, 400);
    assert_eq!(api_json(&response)["error"]["code"], "unknown-project");
}

// --- registry level API used by the portal ------------------------------

#[test]
fn portfolio_row_evidence_summary_is_an_honest_absence() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _) = registered(tmp.path(), "alethefy");
    let registry = Registry::open(&db).unwrap();
    let rows = registry
        .portfolio_fleet(&forge::portfolio::PortfolioFilter::default(), Utc::now())
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].evidence_summary(), "no evidence");
}
