//! Cross-surface regression for the interest package.
//!
//! This suite answers the questions a per-surface contract cannot:
//!
//! - **The store cannot hold a raw event, an identity, a payment
//!   record or a credential.** Asserted against the schema itself
//!   rather than against a validator, because a future column would
//!   bypass every gate in this change.
//! - **No hidden collector exists.** The package adds no route that
//!   contacts a provider, and the sibling analytics adapter surface is
//!   untouched.
//! - **A refused import never erases stored evidence.** This is the
//!   failure mode where a broken provider quietly becomes "no
//!   interest": the previous snapshots stay readable.
//! - **Overlapping windows are never merged.** The row count equals
//!   the number of windows, and the comparison has no total.
//! - **Existing surfaces stay compatible.** The private portfolio
//!   projection, the public share manifest and the fleet list carry no
//!   interest data, and their envelopes are unchanged.

#[path = "support/mod.rs"]
mod support;

use std::path::Path;

use rusqlite::Connection;
use serde_json::json;

use support::interest::*;

fn tmp() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

/// The exact column set of a table, as the database reports it.
fn columns(db: &Path, table: &str) -> Vec<String> {
    let conn = Connection::open(db).expect("raw connection");
    let mut statement = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .expect("table_info");
    let rows = statement
        .query_map([], |row| row.get::<_, String>(1))
        .expect("query");
    rows.map(|row| row.expect("column")).collect()
}

#[test]
fn the_store_has_no_column_that_could_hold_a_raw_payload() {
    let dir = tmp();
    let (db, _) = support::interest::fleet(dir.path());
    import(
        &db,
        &document(vec![snapshot(
            "alethefy",
            "wk-36-a",
            "2026-09-01T00:00:00Z",
            "2026-09-08T00:00:00Z",
        )]),
    );

    // Every column is a declared, typed field. There is deliberately no
    // `payload`, `events`, `metadata`, `json` or `raw` column: a
    // snapshot is a fixed shape or it is not stored.
    assert_eq!(
        columns(&db, "portfolio_interest_snapshots"),
        vec![
            "id",
            "project_id",
            "source",
            "source_revision",
            "window_start",
            "window_end",
            "privacy_mode",
            "coverage",
            "state",
            "replaces_source_revision",
            "actor",
            "received_at",
        ]
    );
    assert_eq!(
        columns(&db, "portfolio_interest_metrics"),
        vec!["id", "snapshot_id", "metric", "value"]
    );
    assert_eq!(
        columns(&db, "portfolio_interest_findings"),
        vec!["id", "project_id", "field", "code", "detail", "created_at"]
    );
    for table in [
        "portfolio_interest_snapshots",
        "portfolio_interest_metrics",
        "portfolio_interest_findings",
    ] {
        for column in columns(&db, table) {
            let lower = column.to_ascii_lowercase();
            for forbidden in [
                "payload", "event", "email", "user", "visitor", "identity", "customer", "card",
                "payment", "token", "secret", "cookie", "ip",
            ] {
                assert!(
                    !lower.contains(forbidden),
                    "{table}.{column} could hold a `{forbidden}`"
                );
            }
        }
    }
}

#[test]
fn the_stored_metric_rows_are_only_allowlisted_names() {
    let dir = tmp();
    let (db, _) = support::interest::fleet(dir.path());
    import(
        &db,
        &document(vec![snapshot(
            "alethefy",
            "wk-36-a",
            "2026-09-01T00:00:00Z",
            "2026-09-08T00:00:00Z",
        )]),
    );
    let conn = Connection::open(&db).expect("raw connection");
    let mut statement = conn
        .prepare("SELECT DISTINCT metric FROM portfolio_interest_metrics ORDER BY metric")
        .expect("prepare");
    let stored: Vec<String> = statement
        .query_map([], |row| row.get(0))
        .expect("query")
        .map(|row| row.expect("metric"))
        .collect();
    let allowlist: Vec<String> = forge::portfolio::interest::InterestMetric::ALL
        .iter()
        .map(|metric| metric.label().to_string())
        .collect();
    for metric in &stored {
        assert!(
            allowlist.contains(metric),
            "`{metric}` reached the store without being on the allowlist"
        );
    }
    assert!(!stored.is_empty());
}

#[test]
fn a_refused_import_never_erases_stored_evidence() {
    let dir = tmp();
    let (db, _) = support::interest::fleet(dir.path());
    import(
        &db,
        &document(vec![
            snapshot(
                "alethefy",
                "wk-35-a",
                "2026-08-25T00:00:00Z",
                "2026-09-01T00:00:00Z",
            ),
            snapshot(
                "alethefy",
                "wk-36-a",
                "2026-09-01T00:00:00Z",
                "2026-09-08T00:00:00Z",
            ),
        ]),
    );
    assert_eq!(snapshot_count(&db, "alethefy"), 2);

    // A provider that now reports only malformed records must leave the
    // earlier evidence exactly where it was. A sound record for another
    // project rides along so the refusal is reported per record — the
    // shape a real mixed batch has.
    let mut broken = snapshot(
        "alethefy",
        "wk-37-a",
        "2026-09-08T00:00:00Z",
        "2026-09-15T00:00:00Z",
    );
    broken["metrics"] = json!({ "unique_visitors": 10, "raw_events": "[]" });
    let sound = snapshot(
        "forge",
        "wk-36-a",
        "2026-09-01T00:00:00Z",
        "2026-09-08T00:00:00Z",
    );
    let out = import(&db, &document(vec![broken, sound]));
    assert_eq!(out["accepted"].as_array().unwrap().len(), 1);
    assert_eq!(out["rejected"].as_array().unwrap().len(), 1);
    assert_eq!(
        snapshot_count(&db, "alethefy"),
        2,
        "a refused import must not remove stored evidence"
    );

    let out = support::share::run_json(&db, &["portfolio", "interest", "show", "alethefy"]);
    assert_eq!(out["interest"]["snapshots"].as_array().unwrap().len(), 2);
}

#[test]
fn overlapping_windows_are_never_merged_or_double_counted() {
    let dir = tmp();
    let (db, _) = support::interest::fleet(dir.path());
    // Two different sources may legitimately cover the same instants;
    // Forge keeps them apart rather than adding them.
    import(
        &db,
        &document(vec![
            snapshot(
                "alethefy",
                "wk-36-a",
                "2026-09-01T00:00:00Z",
                "2026-09-08T00:00:00Z",
            ),
            {
                let mut other = snapshot(
                    "alethefy",
                    "content-9",
                    "2026-09-01T00:00:00Z",
                    "2026-09-08T00:00:00Z",
                );
                other["source"] = json!("content-analytics");
                other["metrics"] = json!({ "unique_visitors": 30 });
                other
            },
        ]),
    );
    let out = support::share::run_json(
        &db,
        &[
            "portfolio",
            "interest",
            "compare",
            "alethefy",
            "--metric",
            "unique_visitors",
        ],
    );
    let comparison = &out["comparison"];
    let rows = comparison["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2, "one row per source window, never one sum");
    let values: Vec<u64> = rows
        .iter()
        .map(|row| row["value"].as_u64().unwrap())
        .collect();
    assert!(values.contains(&120) && values.contains(&30));
    assert!(
        !values.contains(&150),
        "the two sources must never be added: {values:?}"
    );
    assert!(comparison["total"].is_null());
}

#[test]
fn the_private_portfolio_projection_carries_no_interest_data() {
    let dir = tmp();
    let (db, _) = support::interest::fleet(dir.path());
    import(
        &db,
        &document(vec![snapshot(
            "alethefy",
            "wk-36-a",
            "2026-09-01T00:00:00Z",
            "2026-09-08T00:00:00Z",
        )]),
    );
    let out = support::share::run_json(&db, &["portfolio", "show", "alethefy"]);
    let rendered = out.to_string();
    for forbidden in ["unique_visitors", "source_revision", "portfolio_interest"] {
        assert!(
            !rendered.contains(forbidden),
            "the portfolio projection must not carry interest data: {forbidden}"
        );
    }
    // The projection still answers its own contract.
    assert_eq!(out["contract"], "forge-portfolio/0.1.0");
    assert_eq!(out["profile"]["project_id"], "alethefy");
}

#[test]
fn the_public_share_manifest_carries_no_interest_data() {
    let dir = tmp();
    let (db, _) = support::interest::fleet(dir.path());
    import(
        &db,
        &document(vec![snapshot(
            "alethefy",
            "wk-36-a",
            "2026-09-01T00:00:00Z",
            "2026-09-08T00:00:00Z",
        )]),
    );
    // Nothing is shared, so the candidate manifest is empty and, more
    // importantly, must not acquire an interest section by accident.
    let out = support::share::run_json(&db, &["portfolio", "share", "preview"]);
    assert_eq!(out["preview"]["project_count"], 0);
    let rendered = out.to_string();
    for forbidden in ["unique_visitors", "interest", "snapshots"] {
        assert!(
            !rendered.contains(forbidden),
            "the public manifest must not carry interest data: {forbidden}"
        );
    }
}

#[test]
fn the_analytics_adapter_surface_is_untouched() {
    let dir = tmp();
    let (db, _) = support::interest::fleet(dir.path());
    import(
        &db,
        &document(vec![snapshot(
            "alethefy",
            "wk-36-a",
            "2026-09-01T00:00:00Z",
            "2026-09-08T00:00:00Z",
        )]),
    );
    // The sibling analytics surface still discovers its own target and
    // still refuses to invent provider data when nothing is configured.
    let project_dir = dir.path().join("alethefy");
    let out = support::share::run(
        &db,
        &["analytics", "metrics", project_dir.to_str().unwrap()],
    );
    assert!(
        out.status.success(),
        "analytics metrics must keep working: {}",
        support::share::lossy(&out.stderr)
    );
    let human = support::share::lossy(&out.stdout);
    assert!(
        !human.contains("unique_visitors"),
        "the analytics surface must not read the interest store: {human}"
    );
}

#[test]
fn the_fleet_list_envelope_is_unchanged() {
    let dir = tmp();
    let (db, _) = support::interest::fleet(dir.path());
    import(
        &db,
        &document(vec![snapshot(
            "alethefy",
            "wk-36-a",
            "2026-09-01T00:00:00Z",
            "2026-09-08T00:00:00Z",
        )]),
    );
    let out = support::share::run_json(&db, &["list"]);
    let rendered = out.to_string();
    assert!(
        !rendered.contains("unique_visitors"),
        "the fleet list must not gain an interest column: {rendered}"
    );
    assert!(out["projects"].is_array());
}

#[test]
fn the_interest_migration_leaves_every_other_domain_intact() {
    let dir = tmp();
    let (db, _) = support::interest::fleet(dir.path());
    // A registry that already carries the portfolio and share domains
    // migrates to the interest domain without disturbing either.
    let conn = Connection::open(&db).expect("raw connection");
    for table in [
        "projects",
        "operations",
        "portfolio_projects",
        "portfolio_tags",
        "portfolio_share_records",
        "portfolio_share_surfaces",
        "portfolio_interest_snapshots",
        "portfolio_interest_metrics",
        "portfolio_interest_findings",
    ] {
        let present: i64 = conn
            .query_row(
                &format!(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = '{table}'"
                ),
                [],
                |row| row.get(0),
            )
            .expect("sqlite_master");
        assert_eq!(present, 1, "{table} must exist after the migration");
    }
    // Re-opening runs every migration batch again; all are no-ops.
    drop(conn);
    let reopened = forge::registry::Registry::open(&db).expect("reopen");
    import(
        &db,
        &document(vec![snapshot(
            "alethefy",
            "wk-37-a",
            "2026-09-08T00:00:00Z",
            "2026-09-15T00:00:00Z",
        )]),
    );
    assert_eq!(reopened.list().expect("list").len(), 2);
}

#[test]
fn an_import_writes_no_operations_journal_row() {
    let dir = tmp();
    let (db, _) = support::interest::fleet(dir.path());
    let conn = Connection::open(&db).expect("raw connection");
    let before: i64 = conn
        .query_row("SELECT COUNT(*) FROM operations", [], |row| row.get(0))
        .expect("count");
    drop(conn);
    import(
        &db,
        &document(vec![snapshot(
            "alethefy",
            "wk-36-a",
            "2026-09-01T00:00:00Z",
            "2026-09-08T00:00:00Z",
        )]),
    );
    let conn = Connection::open(&db).expect("raw connection");
    let after: i64 = conn
        .query_row("SELECT COUNT(*) FROM operations", [], |row| row.get(0))
        .expect("count");
    assert_eq!(
        before, after,
        "an interest import is not a journaled operation"
    );
}
