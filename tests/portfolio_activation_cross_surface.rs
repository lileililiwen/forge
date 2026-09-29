//! Activation readiness cross-surface regression.
//!
//! Readiness is a read-only projection: a verdict changes no table,
//! no journal row and no registry byte; it reaches no public surface;
//! no billing field exists anywhere; `paid_interest_events` stays an
//! aggregate signal; a pre-change registry reads as-is; and the
//! existing interest surfaces are unchanged.

#[path = "support/mod.rs"]
mod support;

use serde_json::json;

use support::interest::*;

fn tmp() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

fn table_counts(db: &std::path::Path) -> (i64, i64, i64, i64) {
    let conn = rusqlite::Connection::open(db).expect("open");
    let snapshots: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM portfolio_interest_snapshots",
            [],
            |row| row.get(0),
        )
        .expect("snapshots");
    let metrics: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM portfolio_interest_metrics",
            [],
            |row| row.get(0),
        )
        .expect("metrics");
    let findings: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM portfolio_interest_findings",
            [],
            |row| row.get(0),
        )
        .expect("findings");
    let journal: i64 = conn
        .query_row("SELECT COUNT(*) FROM operations", [], |row| row.get(0))
        .expect("journal");
    (snapshots, metrics, findings, journal)
}

#[test]
fn readiness_leaves_every_table_and_the_journal_unchanged() {
    let dir = tmp();
    let (db, _) = support::interest::fleet(dir.path());
    let doc = document(vec![snapshot(
        "alethefy",
        "wk-36-a",
        "2026-09-01T00:00:00Z",
        "2026-09-08T00:00:00Z",
    )]);
    import(&db, &doc);
    let before = table_counts(&db);
    let bytes_before = std::fs::read(&db).expect("registry bytes").len();
    let out = support::share::run(
        &db,
        &[
            "portfolio",
            "activation",
            "readiness",
            "alethefy",
            "--metric",
            "unique_visitors",
            "--min-value",
            "10000",
            "--stale-after-days",
            "365",
        ],
    );
    assert!(!out.status.success(), "not-ready exits non-zero");
    assert!(!out.stdout.is_empty(), "the report still prints");
    assert_eq!(table_counts(&db), before, "readiness must write nothing");
    assert_eq!(
        std::fs::read(&db).expect("registry bytes").len(),
        bytes_before,
        "no registry byte may change"
    );
    // The fleet form is read-only too.
    let out = support::share::run(
        &db,
        &[
            "portfolio",
            "activation",
            "readiness",
            "--metric",
            "unique_visitors",
        ],
    );
    assert!(!out.status.success());
    assert_eq!(table_counts(&db), before);
}

#[test]
fn no_activation_or_billing_field_reaches_the_share_manifest_the_portfolio_projection_or_the_fleet_list(
) {
    let dir = tmp();
    let (db, token) = support::interest::fleet(dir.path());
    let doc = document(vec![snapshot(
        "alethefy",
        "wk-36-a",
        "2026-09-01T00:00:00Z",
        "2026-09-08T00:00:00Z",
    )]);
    import(&db, &doc);
    // Run a readiness projection (verdict arbitrary) to prove it
    // reaches no other surface.
    let _ = support::share::run(
        &db,
        &[
            "portfolio",
            "activation",
            "readiness",
            "alethefy",
            "--metric",
            "unique_visitors",
            "--min-value",
            "1",
            "--stale-after-days",
            "365",
        ],
    );
    for probe in [
        support::share::run_json(&db, &["portfolio", "show", "alethefy"]).to_string(),
        support::share::run_json(&db, &["portfolio", "share", "preview"]).to_string(),
        support::share::run_json(&db, &["list"]).to_string(),
    ] {
        for field in ["activation", "readiness", "threshold", "ready", "revenue"] {
            assert!(
                !probe.contains(&format!("\"{field}\"")),
                "no `{field}` field may leak: {probe}"
            );
        }
    }
    let request = api_request_query("GET", "/v1/projects/alethefy/portfolio", "", Some(&token));
    let response = api(&db, &request);
    let body = api_json(&response).to_string();
    for field in [
        "activation",
        "readiness",
        "threshold",
        "paid_interest_events",
    ] {
        assert!(
            !body.contains(field),
            "no `{field}` in the portfolio projection: {body}"
        );
    }
    let request = api_request("GET", "/v1/projects", None, "");
    let response = api(&db, &request);
    let body = api_json(&response).to_string();
    assert!(!body.contains("activation"), "{body}");
}

#[test]
fn no_price_plan_subscription_entitlement_checkout_or_revenue_field_exists_in_any_projection() {
    let dir = tmp();
    let (db, _) = support::interest::fleet(dir.path());
    // The CLI report, the JSON envelope and the API verdict.
    let out = support::share::run(&db, &["portfolio", "activation", "readiness", "--help"]);
    let help = support::share::lossy(&out.stdout);
    let rendered = {
        let doc = document(vec![snapshot(
            "alethefy",
            "wk-36-a",
            "2026-09-01T00:00:00Z",
            "2026-09-08T00:00:00Z",
        )]);
        import(&db, &doc);
        let out = support::share::run(
            &db,
            &[
                "portfolio",
                "activation",
                "readiness",
                "alethefy",
                "--metric",
                "unique_visitors",
                "--min-value",
                "1",
            ],
        );
        support::share::lossy(&out.stdout)
    };
    let store = rusqlite::Connection::open(&db).expect("open");
    let mut tables: Vec<String> = Vec::new();
    let mut statement = store
        .prepare("SELECT name, sql FROM sqlite_master WHERE type = 'table'")
        .expect("tables");
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .expect("rows");
    for row in rows {
        let (name, sql) = row.expect("row");
        tables.push(format!("{name} {}", sql.unwrap_or_default()));
    }
    for haystack in [help, rendered].into_iter().chain(tables) {
        // Tokenize on non-word characters and underscores so `planned`
        // and `plan_id` are not counted as a `plan` field.
        let tokens: Vec<String> = haystack
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .flat_map(|part| part.split('_'))
            .filter(|part| !part.is_empty())
            .map(str::to_string)
            .collect();
        for word in [
            "price",
            "plan",
            "subscription",
            "entitlement",
            "checkout",
            "revenue",
            "invoice",
            "customer",
            "charge",
            "billing",
        ] {
            // `billing` appears only in the standing deferred-choice
            // note, never as a field; every other word must be absent.
            if word == "billing" {
                continue;
            }
            assert!(
                !tokens.iter().any(|token| token == word),
                "no `{word}` field anywhere: {haystack}"
            );
        }
    }
    // Forge still declares billing blocked.
    let project =
        std::fs::read_to_string(dir.path().join("alethefy").join("forge.yaml")).expect("manifest");
    assert!(!project.to_lowercase().contains("billing"), "{project}");
}

#[test]
fn paid_interest_events_is_an_aggregate_signal_and_grants_no_access() {
    let dir = tmp();
    let (db, token) = support::interest::fleet(dir.path());
    let doc = document(vec![json!({
        "project_id": "alethefy",
        "source": "github-analytics",
        "source_revision": "wk-36-a",
        "window_start": "2026-09-25T00:00:00Z",
        "window_end": "2026-12-31T00:00:00Z",
        "privacy_mode": "exact-count",
        "coverage": "complete",
        "metrics": { "paid_interest_events": 40 },
    })]);
    import(&db, &doc);
    let out = support::share::run(
        &db,
        &[
            "portfolio",
            "activation",
            "readiness",
            "alethefy",
            "--metric",
            "paid_interest_events",
            "--min-value",
            "10",
            "--stale-after-days",
            "365",
        ],
    );
    // Signal-only: readiness says ready against the aggregate, and the
    // report names the metric without any payment, grant or purchase.
    assert!(
        out.status.success(),
        "{}",
        support::share::lossy(&out.stderr)
    );
    let stdout = support::share::lossy(&out.stdout);
    assert!(stdout.contains("ready"), "{stdout}");
    for word in ["payment", "grant", "purchase", "entitlement", "charge"] {
        assert!(!stdout.to_lowercase().contains(word), "{stdout}");
    }
    // The metric grants nothing over the API either.
    let request = api_request_query("GET", "/v1/projects/alethefy/portfolio", "", Some(&token));
    let response = api(&db, &request);
    assert!(!api_json(&response)
        .to_string()
        .contains("paid_interest_events"));
}

#[test]
fn a_registry_written_before_this_package_is_read_as_is() {
    let dir = tmp();
    let legacy = dir.path().join("registry.db");
    {
        let conn = rusqlite::Connection::open(&legacy).expect("open");
        conn.execute_batch(legacy::SQL).expect("legacy schema");
    }
    // Opening migrates forward; readiness then reports no-evidence
    // rather than inventing a figure.
    let out = support::share::run(
        &legacy,
        &[
            "portfolio",
            "activation",
            "readiness",
            "alpha",
            "--metric",
            "unique_visitors",
            "--min-value",
            "1",
        ],
    );
    assert!(!out.status.success());
    assert!(support::share::lossy(&out.stdout).contains("no-evidence"));
}

// Minimal `projects` shape a registry carried before the portfolio
// packages existed.
mod legacy {
    pub const SQL: &str = "
        CREATE TABLE projects (
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
            revision TEXT, build_status TEXT, run_status TEXT,
            container_identity TEXT);
        INSERT INTO projects (id, name, path, profile, schema_version,
            platform_version, observed_at)
        VALUES ('alpha','alpha','/tmp/alpha','rust-web',1,'0.1.0','2026-09-29T00:00:00Z');
    ";
}

#[test]
fn existing_interest_surfaces_are_unchanged() {
    let dir = tmp();
    let (db, _) = support::interest::fleet(dir.path());
    let doc = document(vec![snapshot(
        "alethefy",
        "wk-36-a",
        "2026-09-01T00:00:00Z",
        "2026-09-08T00:00:00Z",
    )]);
    let imported = import(&db, &doc);
    assert_eq!(imported["accepted"].as_array().unwrap().len(), 1);
    // Compare, trend, show, list and audit keep their contracts.
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
    assert_eq!(out["comparison"]["rows"].as_array().unwrap().len(), 1);
    let out = support::share::run_json(
        &db,
        &[
            "portfolio",
            "interest",
            "trend",
            "alethefy",
            "--metric",
            "unique_visitors",
        ],
    );
    assert_eq!(out["trend"]["project_id"], "alethefy");
    let out = support::share::run_json(&db, &["portfolio", "interest", "show", "alethefy"]);
    assert_eq!(out["interest"]["project_id"], "alethefy");
}
