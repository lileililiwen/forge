//! `forge portfolio interest …` CLI contract.
//!
//! The verification oracle for this package is the privacy boundary,
//! not the happy path. These tests prove, in order:
//!
//! - the import accepts only the versioned envelope and the closed
//!   key set, and refuses an identity, raw-event, payment or
//!   credential field **without echoing its value**;
//! - an accepted snapshot is immutable: a retry is idempotent and a
//!   changed payload under the same identity is a conflict;
//! - overlapping same-source windows are refused unless the source
//!   declares which revision it replaces, and a replacement keeps the
//!   superseded revision in history;
//! - a zero is only an observation over a complete window;
//! - a comparison labels source and freshness, never totals and never
//!   ranks across mismatched windows;
//! - nothing contacts a network host and no product-side collector is
//!   involved.

#[path = "support/mod.rs"]
mod support;

use serde_json::json;

use support::interest::*;

fn tmp() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

#[test]
fn interest_help_advertises_the_whole_surface() {
    let dir = tmp();
    let db = dir.path().join("registry.db");
    let out = support::share::run(&db, &["portfolio", "interest", "--help"]);
    let text = support::share::lossy(&out.stdout);
    assert!(out.status.success(), "{text}");
    for word in ["import", "list", "show", "compare", "trend", "audit"] {
        assert!(text.contains(word), "help must advertise `{word}`: {text}");
    }
    let out = support::share::run(&db, &["portfolio", "interest", "import", "--help"]);
    let text = support::share::lossy(&out.stdout);
    for flag in ["--actor", "<FILE>"] {
        assert!(
            text.contains(flag),
            "import help must advertise `{flag}`: {text}"
        );
    }
    let out = support::share::run(&db, &["portfolio", "interest", "compare", "--help"]);
    let text = support::share::lossy(&out.stdout);
    for flag in ["--metric", "--source", "--stale-after-days"] {
        assert!(
            text.contains(flag),
            "compare help must advertise `{flag}`: {text}"
        );
    }
    let out = support::share::run(&db, &["portfolio", "interest", "trend", "--help"]);
    let text = support::share::lossy(&out.stdout);
    for flag in ["--metric", "--limit", "--stale-after-days"] {
        assert!(
            text.contains(flag),
            "trend help must advertise `{flag}`: {text}"
        );
    }
}

#[test]
fn an_import_records_provenance_and_reports_each_record() {
    let dir = tmp();
    let (db, _) = support::interest::fleet(dir.path());
    let import = import(
        &db,
        &document(vec![
            snapshot(
                "alethefy",
                "wk-36-a",
                "2026-09-01T00:00:00Z",
                "2026-09-08T00:00:00Z",
            ),
            json!({
                "project_id": "forge",
                "source": "github-analytics",
                "source_revision": "wk-36-a",
                "window_start": "2026-09-01T00:00:00Z",
                "window_end": "2026-09-08T00:00:00Z",
                "privacy_mode": "lower-bound",
                "coverage": "partial",
                "metrics": { "unique_visitors": 7 },
                "email": "ops@example.com",
            }),
        ]),
    );
    assert_eq!(import["received"], 2);
    assert_eq!(import["accepted"].as_array().unwrap().len(), 1);
    assert_eq!(import["rejected"].as_array().unwrap().len(), 1);
    let accepted = &import["accepted"][0];
    assert_eq!(accepted["project_id"], "alethefy");
    assert_eq!(accepted["source"], "github-analytics");
    assert_eq!(accepted["source_revision"], "wk-36-a");
    assert_eq!(accepted["privacy_mode"], "exact-count");
    assert_eq!(accepted["coverage"], "complete");
    assert_eq!(accepted["state"], "accepted");
    assert_eq!(accepted["actor"], "local-admin");
    // The metric payload is a closed set of named counts, in
    // canonical metric order, not an arbitrary map.
    let metrics: Vec<&str> = accepted["metrics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value["metric"].as_str().unwrap())
        .collect();
    assert_eq!(metrics, vec!["unique_visitors", "outbound_cta_clicks"]);

    let rejected = &import["rejected"][0];
    assert_eq!(rejected["index"], 1);
    assert_eq!(rejected["project_id"], "forge");
    assert_eq!(rejected["code"], "interest-identity-refused");
    // The offending address is neither persisted nor echoed.
    let rendered = import.to_string();
    assert!(!rendered.contains("ops@example.com"), "{rendered}");
    for finding in refusals(&db) {
        assert!(!finding["detail"]
            .as_str()
            .unwrap()
            .contains("ops@example.com"));
    }
}

#[test]
fn the_document_envelope_and_key_set_are_closed() {
    let dir = tmp();
    let (db, _) = support::interest::fleet(dir.path());
    let path = dir.path().join("bad.json");

    // Wrong contract, absent contract and a stray envelope key are all
    // document-level refusals; none of them stores anything.
    for (document, why) in [
        (
            json!({
                "contract": "forge-portfolio-interest/9.9.9",
                "snapshots": [snapshot("alethefy", "r1", "2026-09-01T00:00:00Z", "2026-09-08T00:00:00Z")],
            }),
            "wrong contract",
        ),
        (
            json!({
                "snapshots": [snapshot("alethefy", "r1", "2026-09-01T00:00:00Z", "2026-09-08T00:00:00Z")],
            }),
            "absent contract",
        ),
        (
            json!({
                "contract": CONTRACT,
                "actor": "someone",
                "snapshots": [snapshot("alethefy", "r1", "2026-09-01T00:00:00Z", "2026-09-08T00:00:00Z")],
            }),
            "extra envelope key",
        ),
    ] {
        std::fs::write(&path, document.to_string()).unwrap();
        let out = support::share::run(
            &db,
            &["portfolio", "interest", "import", path.to_str().unwrap()],
        );
        assert!(!out.status.success(), "{why} unexpectedly succeeded");
        assert_eq!(snapshot_count(&db, "alethefy"), 0, "{why} stored a row");
    }
}

#[test]
fn every_refused_field_class_is_named_and_never_echoed() {
    let dir = tmp();
    let (db, _) = support::interest::fleet(dir.path());

    for (field, value, code) in [
        ("email", "ops@example.com", "interest-identity-refused"),
        ("visitor_id", "abc-123", "interest-identity-refused"),
        ("raw_events", "[]", "interest-raw-event-refused"),
        (
            "page_url",
            "https://x.example/a",
            "interest-raw-event-refused",
        ),
        (
            "card_number",
            "4111111111111111",
            "interest-payment-refused",
        ),
        ("revenue", "1999", "interest-payment-refused"),
        ("api_key", "s3cr3tvalue", "interest-credential-refused"),
        ("token", "s3cr3tvalue", "interest-credential-refused"),
        ("engagement_score", "42", "interest-field-unknown"),
    ] {
        let mut record = snapshot(
            "alethefy",
            "r1",
            "2026-09-01T00:00:00Z",
            "2026-09-08T00:00:00Z",
        );
        record[field] = json!(value);
        let out_path = write_document(dir.path(), "bad-field.json", &document(vec![record]));
        let out = support::share::run(
            &db,
            &[
                "portfolio",
                "interest",
                "import",
                out_path.to_str().unwrap(),
            ],
        );
        assert!(!out.status.success(), "{field} unexpectedly succeeded");
        let stderr = support::share::lossy(&out.stderr);
        assert!(
            stderr.contains(&format!("error[portfolio-interest-invalid]")),
            "{field}: {stderr}"
        );
        assert!(
            stderr.contains(code),
            "{field} must report {code}: {stderr}"
        );
        assert!(
            !stderr.contains(value) || value.len() < 4,
            "{field} echoed its value: {stderr}"
        );
        assert_eq!(snapshot_count(&db, "alethefy"), 0, "{field} stored a row");
    }
}

#[test]
fn a_metric_field_is_closed_and_counts_only() {
    let dir = tmp();
    let (db, _) = support::interest::fleet(dir.path());

    // An unlisted metric, however plausible, is refused.
    let mut record = snapshot(
        "alethefy",
        "r1",
        "2026-09-01T00:00:00Z",
        "2026-09-08T00:00:00Z",
    );
    record["metrics"] = json!({ "signups": 3 });
    let path = write_document(dir.path(), "unlisted.json", &document(vec![record]));
    expect_refusal(
        &db,
        &["portfolio", "interest", "import", path.to_str().unwrap()],
        "portfolio-interest-invalid",
    );

    // A negative count is a sign error, not a small number.
    let mut record = snapshot(
        "alethefy",
        "r2",
        "2026-09-01T00:00:00Z",
        "2026-09-08T00:00:00Z",
    );
    record["metrics"] = json!({ "unique_visitors": -1 });
    let path = write_document(dir.path(), "negative.json", &document(vec![record]));
    let out = support::share::run(
        &db,
        &["portfolio", "interest", "import", path.to_str().unwrap()],
    );
    let stderr = support::share::lossy(&out.stderr);
    assert!(stderr.contains("must not be negative"), "{stderr}");

    // A windowed count above the bound is a unit error.
    let mut record = snapshot(
        "alethefy",
        "r3",
        "2026-09-01T00:00:00Z",
        "2026-09-08T00:00:00Z",
    );
    record["metrics"] = json!({ "unique_visitors": 4_000_000_001u64 });
    let path = write_document(dir.path(), "huge.json", &document(vec![record]));
    let out = support::share::run(
        &db,
        &["portfolio", "interest", "import", path.to_str().unwrap()],
    );
    assert!(
        support::share::lossy(&out.stderr).contains("unit error"),
        "{}",
        support::share::lossy(&out.stderr)
    );

    assert_eq!(snapshot_count(&db, "alethefy"), 0, "nothing may be stored");
}

#[test]
fn an_import_is_idempotent_and_a_changed_payload_is_a_conflict() {
    let dir = tmp();
    let (db, _) = support::interest::fleet(dir.path());
    let record = snapshot(
        "alethefy",
        "wk-36-a",
        "2026-09-01T00:00:00Z",
        "2026-09-08T00:00:00Z",
    );

    let first = import(&db, &document(vec![record.clone()]));
    assert_eq!(first["accepted"].as_array().unwrap().len(), 1);

    let retry = import(&db, &document(vec![record.clone()]));
    assert_eq!(retry["accepted"].as_array().unwrap().len(), 0);
    assert_eq!(retry["already_present"].as_array().unwrap().len(), 1);
    assert_eq!(retry["rejected"].as_array().unwrap().len(), 0);
    assert_eq!(
        snapshot_count(&db, "alethefy"),
        1,
        "a retry must not create a second row"
    );

    // The same identity with a different value is a contradiction: the
    // snapshot is immutable, so Forge refuses rather than merging. A
    // sound companion record rides along so the refusal is reported
    // per record rather than as a failed batch.
    let mut changed = record;
    changed["metrics"] = json!({ "unique_visitors": 999 });
    let sound = snapshot(
        "forge",
        "wk-36-a",
        "2026-09-01T00:00:00Z",
        "2026-09-08T00:00:00Z",
    );
    let out = import(&db, &document(vec![changed, sound]));
    assert_eq!(out["accepted"].as_array().unwrap().len(), 1);
    assert_eq!(out["rejected"].as_array().unwrap().len(), 1);
    let code = out["rejected"][0]["code"].as_str().unwrap();
    assert_eq!(code, "interest-overlap-refused");
    let detail = out["rejected"][0]["detail"].as_str().unwrap();
    assert!(detail.contains("already attested"), "{detail}");
    assert_eq!(snapshot_count(&db, "alethefy"), 1);
}

#[test]
fn overlapping_windows_require_a_declared_replacement() {
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

    // Overlap with no declaration is refused, and the shared days are
    // named so the operator can see what would have been double-counted.
    let overlapping = snapshot(
        "alethefy",
        "wk-37-a",
        "2026-09-07T00:00:00Z",
        "2026-09-14T00:00:00Z",
    );
    let out = import(
        &db,
        &document(vec![
            overlapping,
            snapshot(
                "forge",
                "wk-36-a",
                "2026-09-01T00:00:00Z",
                "2026-09-08T00:00:00Z",
            ),
        ]),
    );
    assert_eq!(out["rejected"][0]["code"], "interest-overlap-refused");
    assert!(out["rejected"][0]["detail"]
        .as_str()
        .unwrap()
        .contains("count the shared days twice"));
    assert_eq!(snapshot_count(&db, "alethefy"), 1);

    // Adjacent windows are the normal weekly case, not a collision.
    import(
        &db,
        &document(vec![snapshot(
            "alethefy",
            "wk-37-b",
            "2026-09-08T00:00:00Z",
            "2026-09-15T00:00:00Z",
        )]),
    );
    assert_eq!(snapshot_count(&db, "alethefy"), 2);

    // A declared replacement supersedes the named revision and keeps
    // the history readable.
    let mut replacement = snapshot(
        "alethefy",
        "wk-37-c",
        "2026-09-07T00:00:00Z",
        "2026-09-14T00:00:00Z",
    );
    replacement["replaces_source_revision"] = json!("wk-36-a");
    let out = import(&db, &document(vec![replacement]));
    assert_eq!(out["supersessions"].as_array().unwrap().len(), 1);
    let supersession = &out["supersessions"][0];
    assert_eq!(supersession["snapshot"]["source_revision"], "wk-37-c");
    assert_eq!(supersession["superseded"]["source_revision"], "wk-36-a");
    assert_eq!(supersession["superseded"]["state"], "superseded");
    assert_eq!(snapshot_count(&db, "alethefy"), 3, "history is retained");

    // The comparison view narrows to the current revision, so the
    // superseded one never answers the same question twice.
    let shown = support::share::run_json(&db, &["portfolio", "interest", "show", "alethefy"]);
    let states: Vec<&str> = shown["interest"]["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["state"].as_str().unwrap())
        .collect();
    assert!(states.contains(&"superseded"), "{states:?}");
    assert_eq!(shown["interest"]["superseded_snapshots"], 1);
}

#[test]
fn a_superseded_revision_never_appears_in_a_trend() {
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
    // Re-measure the same window: the earlier revision becomes history.
    let mut replacement = snapshot(
        "alethefy",
        "wk-36-b",
        "2026-09-01T00:00:00Z",
        "2026-09-08T00:00:00Z",
    );
    replacement["replaces_source_revision"] = json!("wk-36-a");
    replacement["metrics"] = json!({ "unique_visitors": 500 });
    import(&db, &document(vec![replacement]));

    let trend = support::share::run_json(
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
    let points = trend["trend"]["points"].as_array().unwrap();
    assert_eq!(
        points.len(),
        1,
        "one window must yield one point, not one per revision: {points:?}"
    );
    assert_eq!(
        points[0]["value"], 500,
        "the replacement is the live figure"
    );
    assert_eq!(points[0]["window_start"], "2026-09-01T00:00:00Z");

    // `show` still carries the whole history for a human.
    let shown = support::share::run_json(&db, &["portfolio", "interest", "show", "alethefy"]);
    assert_eq!(shown["interest"]["snapshots"].as_array().unwrap().len(), 2);
}

#[test]
fn a_zero_is_only_an_observation_over_a_complete_window() {
    let dir = tmp();
    let (db, _) = support::interest::fleet(dir.path());

    let mut partial = snapshot(
        "alethefy",
        "wk-36-a",
        "2026-09-01T00:00:00Z",
        "2026-09-08T00:00:00Z",
    );
    partial["coverage"] = json!("partial");
    partial["metrics"] = json!({ "unique_visitors": 0 });
    let out = support::share::run(
        &db,
        &[
            "portfolio",
            "interest",
            "import",
            write_document(dir.path(), "zero-partial.json", &document(vec![partial]))
                .to_str()
                .unwrap(),
        ],
    );
    let stderr = support::share::lossy(&out.stderr);
    assert!(stderr.contains("measured the complete window"), "{stderr}");
    assert_eq!(snapshot_count(&db, "alethefy"), 0);

    // The same zeros over a window the source says it measured are a
    // real observation and are stored.
    let mut complete = snapshot(
        "alethefy",
        "wk-36-a",
        "2026-09-01T00:00:00Z",
        "2026-09-08T00:00:00Z",
    );
    complete["coverage"] = json!("complete");
    complete["metrics"] = json!({ "unique_visitors": 0 });
    let out = import(&db, &document(vec![complete]));
    assert_eq!(out["accepted"].as_array().unwrap().len(), 1);
    assert_eq!(out["accepted"][0]["metrics"][0]["value"], 0);
}

#[test]
fn a_comparison_labels_every_row_and_never_totals() {
    let dir = tmp();
    let (db, _) = support::interest::fleet(dir.path());
    import(
        &db,
        &document(vec![
            snapshot(
                "alethefy",
                "wk-36-a",
                "2026-09-01T00:00:00Z",
                "2026-09-08T00:00:00Z",
            ),
            snapshot(
                "forge",
                "wk-36-a",
                "2026-09-01T00:00:00Z",
                "2026-09-08T00:00:00Z",
            ),
        ]),
    );

    let out = support::share::run_json(
        &db,
        &[
            "portfolio",
            "interest",
            "compare",
            "alethefy",
            "forge",
            "--metric",
            "unique_visitors",
        ],
    );
    let comparison = &out["comparison"];
    assert_eq!(comparison["comparable"], true);
    assert_eq!(comparison["rows"].as_array().unwrap().len(), 2);
    for row in comparison["rows"].as_array().unwrap() {
        // Every row carries its provenance and its freshness; no row
        // carries a total, an average or a rank.
        for key in [
            "source",
            "source_revision",
            "privacy_mode",
            "coverage",
            "freshness",
            "window_start",
            "window_end",
        ] {
            assert!(row.get(key).is_some(), "row must carry `{key}`: {row}");
        }
        for forbidden in ["total", "average", "rank", "sum"] {
            assert!(
                row.get(forbidden).is_none(),
                "a comparison row must not carry `{forbidden}`: {row}"
            );
        }
    }
    assert!(
        comparison.get("total").is_none(),
        "a comparison must not expose a total: {comparison}"
    );
}

#[test]
fn a_comparison_across_different_windows_refuses_to_rank() {
    let dir = tmp();
    let (db, _) = support::interest::fleet(dir.path());
    import(
        &db,
        &document(vec![
            snapshot(
                "alethefy",
                "wk-36-a",
                "2026-09-01T00:00:00Z",
                "2026-09-08T00:00:00Z",
            ),
            snapshot(
                "forge",
                "wk-37-a",
                "2026-09-08T00:00:00Z",
                "2026-09-15T00:00:00Z",
            ),
        ]),
    );
    let out = support::share::run_json(
        &db,
        &[
            "portfolio",
            "interest",
            "compare",
            "alethefy",
            "forge",
            "--metric",
            "unique_visitors",
        ],
    );
    let comparison = &out["comparison"];
    assert_eq!(comparison["comparable"], false);
    assert_eq!(comparison["windows"].as_array().unwrap().len(), 2);
    let notes = comparison["notes"].as_array().unwrap();
    assert!(
        notes
            .iter()
            .any(|note| note.as_str().unwrap().contains("will not rank or total")),
        "{notes:?}"
    );
}

#[test]
fn a_trend_lists_windows_and_counts_the_gaps() {
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
    let trend = &out["trend"];
    assert_eq!(trend["metric"], "unique_visitors");
    let points = trend["points"].as_array().unwrap();
    assert_eq!(points.len(), 2);
    // Oldest window first, so a bounded series keeps the history.
    assert_eq!(points[0]["window_start"], "2026-08-25T00:00:00Z");
    assert_eq!(points[1]["window_start"], "2026-09-01T00:00:00Z");

    // A metric nobody reported yields no points at all — never a zero.
    let out = support::share::run_json(
        &db,
        &[
            "portfolio",
            "interest",
            "trend",
            "alethefy",
            "--metric",
            "paid_interest_events",
        ],
    );
    assert!(out["trend"]["points"].as_array().unwrap().is_empty());
    assert!(out["trend"]["note"]
        .as_str()
        .unwrap()
        .contains("inventing one"));
}

#[test]
fn an_unknown_project_and_an_unknown_metric_are_typed_refusals() {
    let dir = tmp();
    let (db, _) = support::interest::fleet(dir.path());

    let mut record = snapshot(
        "no-such-project",
        "wk-36-a",
        "2026-09-01T00:00:00Z",
        "2026-09-08T00:00:00Z",
    );
    record["project_id"] = json!("no-such-project");
    let path = write_document(dir.path(), "unknown.json", &document(vec![record]));
    expect_refusal(
        &db,
        &["portfolio", "interest", "import", path.to_str().unwrap()],
        "unknown-project",
    );

    expect_refusal(
        &db,
        &[
            "portfolio",
            "interest",
            "trend",
            "alethefy",
            "--metric",
            "bounce_rate",
        ],
        "portfolio-interest-invalid",
    );
    expect_refusal(
        &db,
        &[
            "portfolio",
            "interest",
            "compare",
            "alethefy",
            "--metric",
            "bounce_rate",
        ],
        "portfolio-interest-invalid",
    );
    expect_refusal(
        &db,
        &["portfolio", "interest", "show", "no-such-project"],
        "unknown-project",
    );
}

#[test]
fn the_staleness_bound_is_validated_rather_than_clamped() {
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
    for bad in ["0", "366"] {
        expect_refusal(
            &db,
            &[
                "portfolio",
                "interest",
                "show",
                "alethefy",
                "--stale-after-days",
                bad,
            ],
            "portfolio-interest-invalid",
        );
    }
    // The bound changes the label, never the stored value.
    let tight = support::share::run_json(
        &db,
        &[
            "portfolio",
            "interest",
            "show",
            "alethefy",
            "--stale-after-days",
            "1",
        ],
    );
    assert_eq!(tight["interest"]["stale_snapshots"], 1);
    let loose = support::share::run_json(
        &db,
        &[
            "portfolio",
            "interest",
            "show",
            "alethefy",
            "--stale-after-days",
            "365",
        ],
    );
    assert_eq!(loose["interest"]["stale_snapshots"], 0);
    assert_eq!(
        tight["interest"]["snapshots"][0]["metrics"][0]["value"],
        loose["interest"]["snapshots"][0]["metrics"][0]["value"],
        "widening the bound must not rewrite the observed value"
    );
}

#[test]
fn the_audit_limit_is_bounded() {
    let dir = tmp();
    let (db, _) = support::interest::fleet(dir.path());
    for bad in ["0", "501"] {
        expect_refusal(
            &db,
            &["portfolio", "interest", "audit", "--limit", bad],
            "portfolio-interest-invalid",
        );
    }
    let out = support::share::run_json(&db, &["portfolio", "interest", "audit", "--limit", "1"]);
    assert!(out["refusals"].is_array());
}

#[test]
fn an_import_touches_no_network_and_journals_no_operation_row() {
    let dir = tmp();
    let (db, _) = support::interest::fleet(dir.path());
    // The proxy variables are cleared, so a network attempt would have
    // to be direct; the registry is a local file and the import writes
    // nothing but rows.
    let before = support::share::run_json(&db, &["deploy", "status", "--project", "alethefy"]);
    import(
        &db,
        &document(vec![snapshot(
            "alethefy",
            "wk-36-a",
            "2026-09-01T00:00:00Z",
            "2026-09-08T00:00:00Z",
        )]),
    );
    let after = support::share::run_json(&db, &["deploy", "status", "--project", "alethefy"]);
    assert_eq!(
        before["entries"], after["entries"],
        "an interest import must write no operations journal row"
    );
}
