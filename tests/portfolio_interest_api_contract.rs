//! JSON API contract for aggregate interest evidence.
//!
//! The HTTP transport adds no rule of its own: every handler dispatches
//! into the same orchestration the CLI uses. These tests prove the
//! transport-level properties the CLI cannot express:
//!
//! - every interest route, reads included, demands a live admin
//!   session, and an unauthorized request persists nothing;
//! - a session minted for one project cannot import into another;
//! - the importer recorded is the *authenticated subject*, never a
//!   client-claimed actor;
//! - a refused record is a per-record rejection over HTTP too, and a
//!   partial batch still stores its sound records;
//! - query parameters are bounded and typed rather than silently
//!   defaulted.

#[path = "support/mod.rs"]
mod support;

use serde_json::json;

use support::interest::*;

fn tmp() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

#[test]
fn every_interest_route_demands_a_session() {
    let dir = tmp();
    let (db, _) = support::interest::fleet(dir.path());
    let routes: Vec<(&str, &str)> = vec![
        ("GET", "/v1/projects/alethefy/interest"),
        ("POST", "/v1/projects/alethefy/interest"),
        ("GET", "/v1/interest/compare"),
        ("GET", "/v1/interest/trend"),
        ("GET", "/v1/interest/audit"),
    ];
    for (method, path) in routes {
        let request = api_request(method, path, None, "{}");
        let response = api(&db, &request);
        expect_api_error(&response, 401, "api-unauthorized");
    }
    // Nothing was written by any of the unauthorized calls.
    assert_eq!(snapshot_count(&db, "alethefy"), 0);
    assert!(refusals(&db).is_empty());
}

#[test]
fn a_bad_bearer_is_refused_before_any_rule_runs() {
    let dir = tmp();
    let (db, _) = support::interest::fleet(dir.path());
    for token in ["", "not-hex", "zz"] {
        let request = api_request("GET", "/v1/projects/alethefy/interest", Some(token), "");
        let response = api(&db, &request);
        expect_api_error(&response, 401, "api-unauthorized");
    }
}

#[test]
fn the_api_import_records_the_session_subject_as_provenance() {
    let dir = tmp();
    let (db, token) = support::interest::fleet(dir.path());
    let response = import_api(
        &db,
        &token,
        "alethefy",
        vec![snapshot(
            "alethefy",
            "wk-36-a",
            "2026-09-01T00:00:00Z",
            "2026-09-08T00:00:00Z",
        )],
    );
    assert_eq!(response.status, 200, "{}", api_json(&response));
    let body = api_json(&response);
    assert_eq!(body["contract"], CONTRACT);
    assert_eq!(
        body["interest"]["import"]["accepted"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    // The actor is the authenticated subject, not a body field.
    assert_eq!(body["interest"]["import"]["actor"], "ops-admin");
    assert_eq!(
        body["interest"]["import"]["accepted"][0]["actor"],
        "ops-admin"
    );
}

#[test]
fn a_cross_project_session_cannot_import_into_another_project() {
    let dir = tmp();
    let (db, token) = support::interest::fleet(dir.path());
    let response = import_api(
        &db,
        &token,
        "forge",
        vec![snapshot(
            "forge",
            "wk-36-a",
            "2026-09-01T00:00:00Z",
            "2026-09-08T00:00:00Z",
        )],
    );
    expect_api_error(&response, 403, "api-project-mismatch");
    assert_eq!(snapshot_count(&db, "forge"), 0, "no state may change");
}

#[test]
fn a_body_naming_another_project_is_a_typed_400() {
    let dir = tmp();
    let (db, token) = support::interest::fleet(dir.path());
    // The route targets `alethefy`; the record claims `forge`.
    let response = import_api(
        &db,
        &token,
        "alethefy",
        vec![json!({
            "project_id": "forge",
            "source": "github-analytics",
            "source_revision": "wk-36-a",
            "window_start": "2026-09-01T00:00:00Z",
            "window_end": "2026-09-08T00:00:00Z",
            "coverage": "complete",
            "metrics": { "unique_visitors": 5 },
        })],
    );
    expect_api_error(&response, 400, "api-invalid");
    assert_eq!(snapshot_count(&db, "alethefy"), 0);
    assert_eq!(snapshot_count(&db, "forge"), 0);
}

#[test]
fn an_absent_snapshots_array_is_a_typed_400() {
    let dir = tmp();
    let (db, token) = support::interest::fleet(dir.path());
    for body in [
        json!({}),
        json!({ "snapshots": "nope" }),
        json!({ "snapshots": {} }),
    ] {
        let request = api_request(
            "POST",
            "/v1/projects/alethefy/interest",
            Some(&token),
            &body.to_string(),
        );
        let response = api(&db, &request);
        expect_api_error(&response, 400, "api-invalid");
    }
}

#[test]
fn a_refused_record_is_per_record_over_http_and_the_sound_ones_still_store() {
    let dir = tmp();
    let (db, token) = support::interest::fleet(dir.path());
    let response = import_api(
        &db,
        &token,
        "alethefy",
        vec![
            snapshot(
                "alethefy",
                "wk-36-a",
                "2026-09-01T00:00:00Z",
                "2026-09-08T00:00:00Z",
            ),
            json!({
                "project_id": "alethefy",
                "source": "github-analytics",
                "source_revision": "wk-36-b",
                "window_start": "2026-09-08T00:00:00Z",
                "window_end": "2026-09-15T00:00:00Z",
                "coverage": "complete",
                "metrics": { "unique_visitors": 3, "visitor_id": "visitor-abc-123" },
            }),
        ],
    );
    assert_eq!(response.status, 200, "{}", api_json(&response));
    let body = api_json(&response);
    let import = &body["interest"]["import"];
    assert_eq!(import["accepted"].as_array().unwrap().len(), 1);
    assert_eq!(import["rejected"].as_array().unwrap().len(), 1);
    assert_eq!(import["rejected"][0]["code"], "interest-identity-refused");
    // The identity value is neither echoed nor persisted.
    let rendered = body.to_string();
    assert!(!rendered.contains("visitor-abc-123"), "{rendered}");
    for finding in refusals(&db) {
        assert!(!finding["detail"]
            .as_str()
            .unwrap()
            .contains("visitor-abc-123"));
    }
    assert_eq!(snapshot_count(&db, "alethefy"), 1);
}

#[test]
fn an_overlap_over_http_is_reported_per_record() {
    let dir = tmp();
    let (db, token) = support::interest::fleet(dir.path());
    import_api(
        &db,
        &token,
        "alethefy",
        vec![snapshot(
            "alethefy",
            "wk-36-a",
            "2026-09-01T00:00:00Z",
            "2026-09-08T00:00:00Z",
        )],
    );
    let response = import_api(
        &db,
        &token,
        "alethefy",
        vec![snapshot(
            "alethefy",
            "wk-37-a",
            "2026-09-07T00:00:00Z",
            "2026-09-14T00:00:00Z",
        )],
    );
    // A single-record batch whose only record overlaps is reported per
    // record, so the transport answers 200 with the rejection.
    assert_eq!(response.status, 200, "{}", api_json(&response));
    let body = api_json(&response);
    assert_eq!(
        body["interest"]["import"]["rejected"][0]["code"],
        "interest-overlap-refused"
    );
    assert_eq!(snapshot_count(&db, "alethefy"), 1);
}

#[test]
fn the_read_routes_answer_their_projection() {
    let dir = tmp();
    let (db, token) = support::interest::fleet(dir.path());
    import_api(
        &db,
        &token,
        "alethefy",
        vec![snapshot(
            "alethefy",
            "wk-36-a",
            "2026-09-01T00:00:00Z",
            "2026-09-08T00:00:00Z",
        )],
    );

    let request = api_request("GET", "/v1/projects/alethefy/interest", Some(&token), "");
    let response = api(&db, &request);
    assert_eq!(response.status, 200, "{}", api_json(&response));
    let body = api_json(&response);
    assert_eq!(body["interest"]["project_id"], "alethefy");
    assert_eq!(body["interest"]["measured"], true);
    assert_eq!(
        body["interest"]["projection"]["snapshots"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let request = api_request_query(
        "GET",
        "/v1/interest/compare",
        "projects=alethefy&metric=unique_visitors",
        Some(&token),
    );
    let response = api(&db, &request);
    assert_eq!(response.status, 200, "{}", api_json(&response));
    let comparison = &api_json(&response)["interest"]["comparison"];
    assert_eq!(comparison["rows"].as_array().unwrap().len(), 1);
    assert!(comparison["total"].is_null());

    let request = api_request_query(
        "GET",
        "/v1/interest/trend",
        "project=alethefy&metric=unique_visitors",
        Some(&token),
    );
    let response = api(&db, &request);
    assert_eq!(response.status, 200, "{}", api_json(&response));
    let trend = &api_json(&response)["interest"]["trend"];
    assert_eq!(trend["points"].as_array().unwrap().len(), 1);

    let request = api_request("GET", "/v1/interest/audit", Some(&token), "");
    let response = api(&db, &request);
    assert_eq!(response.status, 200, "{}", api_json(&response));
    assert!(api_json(&response)["interest"]["refusals"].is_array());
}

#[test]
fn an_unmeasured_project_says_so_rather_than_inventing_a_zero() {
    let dir = tmp();
    let (db, token) = support::interest::fleet(dir.path());
    // No snapshot has been imported: the projection must say "not
    // measured" rather than presenting a zero the source never made.
    let request = api_request("GET", "/v1/projects/alethefy/interest", Some(&token), "");
    let response = api(&db, &request);
    assert_eq!(response.status, 200, "{}", api_json(&response));
    let body = api_json(&response);
    assert_eq!(body["interest"]["measured"], false);
    assert!(body["interest"]["projection"]["snapshots"]
        .as_array()
        .unwrap()
        .is_empty());
    // A read-only projection refusals list is scoped to the project.
    assert!(body["interest"]["projection"]["findings"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn interest_query_parameters_are_bounded_and_typed() {
    let dir = tmp();
    let (db, token) = support::interest::fleet(dir.path());

    for (query, why) in [
        ("projects=", "no projects named"),
        ("metric=unique_visitors", "no projects parameter"),
        ("projects=alethefy&bogus=1", "unknown parameter"),
    ] {
        let request = api_request_query("GET", "/v1/interest/compare", query, Some(&token));
        let response = api(&db, &request);
        expect_api_error(&response, 400, "api-invalid");
        let _ = why;
    }

    // An unknown metric is a typed refusal rather than a silently
    // dropped column.
    let request = api_request_query(
        "GET",
        "/v1/interest/compare",
        "projects=alethefy&metric=bounce_rate",
        Some(&token),
    );
    let response = api(&db, &request);
    expect_api_error(&response, 400, "api-invalid");

    let request = api_request_query(
        "GET",
        "/v1/interest/trend",
        "project=alethefy&metric=bounce_rate",
        Some(&token),
    );
    let response = api(&db, &request);
    expect_api_error(&response, 400, "api-invalid");

    // A comparison of more projects than the bound is refused rather
    // than silently narrowed.
    let many: Vec<String> = (0..40).map(|index| format!("p{index}")).collect();
    let request = api_request_query(
        "GET",
        "/v1/interest/compare",
        &format!("projects={}", many.join(",")),
        Some(&token),
    );
    let response = api(&db, &request);
    expect_api_error(&response, 400, "portfolio-interest-invalid");

    // The staleness bound is bounded, not clamped.
    for bad in ["0", "366", "not-a-number"] {
        let request = api_request_query(
            "GET",
            "/v1/projects/alethefy/interest",
            &format!("stale_after_days={bad}"),
            Some(&token),
        );
        let response = api(&db, &request);
        expect_api_error(&response, 400, "api-invalid");
    }

    for bad in ["0", "501"] {
        let request = api_request_query(
            "GET",
            "/v1/interest/audit",
            &format!("limit={bad}"),
            Some(&token),
        );
        let response = api(&db, &request);
        expect_api_error(&response, 400, "api-invalid");
    }
}

#[test]
fn an_unknown_project_is_a_typed_refusal_on_every_read_route() {
    let dir = tmp();
    let (db, token) = support::interest::fleet(dir.path());
    let request = api_request(
        "GET",
        "/v1/projects/no-such-project/interest",
        Some(&token),
        "",
    );
    let response = api(&db, &request);
    expect_api_error(&response, 400, "unknown-project");

    let request = api_request_query(
        "GET",
        "/v1/interest/compare",
        "projects=alethefy,no-such-project&metric=unique_visitors",
        Some(&token),
    );
    let response = api(&db, &request);
    expect_api_error(&response, 400, "unknown-project");

    let request = api_request_query(
        "GET",
        "/v1/interest/trend",
        "project=no-such-project&metric=unique_visitors",
        Some(&token),
    );
    let response = api(&db, &request);
    expect_api_error(&response, 400, "unknown-project");
}

#[test]
fn the_health_and_project_routes_stay_unchanged() {
    let dir = tmp();
    let (db, token) = support::interest::fleet(dir.path());
    // `/healthz` never needed a session and still does not.
    let request = api_request("GET", "/healthz", None, "");
    let response = api(&db, &request);
    assert_eq!(response.status, 200);
    // `/v1/projects` still demands a real session.
    let request = api_request("GET", "/v1/projects", None, "");
    let response = api(&db, &request);
    expect_api_error(&response, 401, "api-unauthorized");
    let request = api_request("GET", "/v1/projects", Some(&token), "");
    let response = api(&db, &request);
    assert_eq!(response.status, 200);
}
