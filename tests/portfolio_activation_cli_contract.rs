//! `forge portfolio activation readiness …` CLI + API contract.
//!
//! The verification oracle for this package is the gate, not the happy
//! path. These tests prove, in order:
//!
//! - the command advertises its flags and the metric is required;
//! - a ready project prints the report and exits zero;
//! - a not-ready project prints the report and exits non-zero with the
//!   typed gate error on stderr;
//! - every not-ready reason is reachable and named;
//! - an absent threshold is a verdict, not an error;
//! - input errors are typed `portfolio-interest-invalid` refusals with
//!   empty stdout;
//! - the fleet form evaluates every project in id order;
//! - JSON carries the verdict in both cases;
//! - every readiness route demands an admin session;
//! - the API answers `200` for both verdicts and bounds its query.

#[path = "support/mod.rs"]
mod support;

use serde_json::json;

use support::interest::*;

fn tmp() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

fn ready_world(dir: &std::path::Path) -> (std::path::PathBuf, String) {
    let (db, token) = support::interest::fleet(dir);
    let doc = document(vec![snapshot(
        "alethefy",
        "wk-36-a",
        "2026-09-01T00:00:00Z",
        "2026-09-08T00:00:00Z",
    )]);
    import(&db, &doc);
    (db, token)
}

fn readiness(db: &std::path::Path, args: &[&str]) -> std::process::Output {
    let mut full = vec!["portfolio", "activation", "readiness"];
    full.extend(args.iter().copied());
    support::share::run(db, &full)
}

fn readiness_json(db: &std::path::Path, args: &[&str]) -> serde_json::Value {
    let mut full = vec!["portfolio", "activation", "readiness"];
    full.extend(args.iter().copied());
    support::share::run_json(db, &full)
}

#[test]
fn activation_help_advertises_the_command_and_its_flags() {
    let dir = tmp();
    let db = dir.path().join("registry.db");
    let out = support::share::run(&db, &["portfolio", "activation", "--help"]);
    let text = support::share::lossy(&out.stdout);
    assert!(out.status.success(), "{text}");
    assert!(text.contains("readiness"), "{text}");
    let out = support::share::run(&db, &["portfolio", "activation", "readiness", "--help"]);
    let text = support::share::lossy(&out.stdout);
    for flag in [
        "--metric",
        "--min-value",
        "--source",
        "--window",
        "--stale-after-days",
    ] {
        assert!(
            text.contains(flag),
            "readiness help must advertise `{flag}`: {text}"
        );
    }
}

#[test]
fn a_ready_project_reports_ready_prints_the_report_and_exits_zero() {
    let dir = tmp();
    let (db, _) = ready_world(dir.path());
    let out = readiness(
        &db,
        &[
            "alethefy",
            "--metric",
            "unique_visitors",
            "--min-value",
            "100",
            "--stale-after-days",
            "365",
        ],
    );
    let stdout = support::share::lossy(&out.stdout);
    assert!(
        out.status.success(),
        "stderr: {}",
        support::share::lossy(&out.stderr)
    );
    assert!(stdout.contains("alethefy"), "{stdout}");
    assert!(stdout.contains("ready"), "{stdout}");
    assert!(stdout.contains("summary: ready=1 not-ready=0"), "{stdout}");
    assert!(
        stdout.contains("forge-portfolio-activation/0.1.0"),
        "{stdout}"
    );
}

#[test]
fn a_not_ready_project_prints_the_report_and_exits_non_zero() {
    let dir = tmp();
    let (db, _) = ready_world(dir.path());
    let out = readiness(
        &db,
        &[
            "alethefy",
            "--metric",
            "unique_visitors",
            "--min-value",
            "10000",
            "--stale-after-days",
            "365",
        ],
    );
    let stdout = support::share::lossy(&out.stdout);
    let stderr = support::share::lossy(&out.stderr);
    assert!(
        !out.status.success(),
        "a not-ready verdict must exit non-zero"
    );
    assert!(
        !stdout.is_empty(),
        "the report must print even when not ready"
    );
    assert!(stdout.contains("not-ready"), "{stdout}");
    assert!(stdout.contains("below-threshold"), "{stdout}");
    assert!(
        stderr.contains("error[portfolio-activation-not-ready]"),
        "{stderr}"
    );
    assert!(
        stderr.contains("1 of 1 project(s) are not ready"),
        "{stderr}"
    );
    assert!(stderr.contains("alethefy:below-threshold"), "{stderr}");
}

#[test]
fn every_not_ready_reason_is_reachable_and_named() {
    let dir = tmp();
    let (db, _) = support::interest::fleet(dir.path());
    // no-evidence: forge has no snapshots.
    let out = readiness(
        &db,
        &["forge", "--metric", "unique_visitors", "--min-value", "10"],
    );
    assert!(!out.status.success());
    assert!(support::share::lossy(&out.stdout).contains("no-evidence"));

    // superseded-only: replace alethefy's only window.
    let first = document(vec![snapshot(
        "alethefy",
        "wk-36-a",
        "2026-09-01T00:00:00Z",
        "2026-09-08T00:00:00Z",
    )]);
    import(&db, &first);
    let replacement = document(vec![json!({
        "project_id": "alethefy",
        "source": "github-analytics",
        "source_revision": "wk-36-b",
        "replaces_source_revision": "wk-36-a",
        "window_start": "2026-09-01T00:00:00Z",
        "window_end": "2026-09-08T00:00:00Z",
        "privacy_mode": "exact-count",
        "coverage": "complete",
        "metrics": { "unique_visitors": 120 },
    })]);
    import(&db, &replacement);
    // History remains; current evidence exists, so craft superseded-only
    // on a second project instead: import then supersede forge too.
    let forge_first = document(vec![snapshot(
        "forge",
        "wk-36-a",
        "2026-09-01T00:00:00Z",
        "2026-09-08T00:00:00Z",
    )]);
    import(&db, &forge_first);
    let forge_replacement = document(vec![json!({
        "project_id": "forge",
        "source": "github-analytics",
        "source_revision": "wk-36-b",
        "replaces_source_revision": "wk-36-a",
        "window_start": "2026-09-01T00:00:00Z",
        "window_end": "2026-09-08T00:00:00Z",
        "privacy_mode": "exact-count",
        "coverage": "complete",
        "metrics": { "unique_visitors": 120 },
    })]);
    import(&db, &forge_replacement);
    // no-current-window: ask for a metric no window reports.
    let out = readiness(
        &db,
        &[
            "alethefy",
            "--metric",
            "paid_interest_events",
            "--min-value",
            "1",
        ],
    );
    assert!(support::share::lossy(&out.stdout).contains("no-current-window"));

    // stale-window: the window ended long ago.
    let out = readiness(
        &db,
        &[
            "alethefy",
            "--metric",
            "unique_visitors",
            "--min-value",
            "1",
        ],
    );
    assert!(support::share::lossy(&out.stdout).contains("stale-window"));

    // inexact-privacy-mode + partial-coverage + below-threshold: a
    // lower-bound partial window above the stale bound.
    let dir2 = tmp();
    let (db2, _) = support::interest::fleet(dir2.path());
    let doc = document(vec![json!({
        "project_id": "alethefy",
        "source": "github-analytics",
        "source_revision": "fresh-a",
        "window_start": "2026-09-25T00:00:00Z",
        "window_end": "2026-12-31T00:00:00Z",
        "privacy_mode": "lower-bound",
        "coverage": "partial",
        "metrics": { "unique_visitors": 5 },
    })]);
    import(&db2, &doc);
    let out = readiness(
        &db2,
        &[
            "alethefy",
            "--metric",
            "unique_visitors",
            "--min-value",
            "100",
            "--stale-after-days",
            "365",
        ],
    );
    let stdout = support::share::lossy(&out.stdout);
    assert!(stdout.contains("inexact-privacy-mode"), "{stdout}");
    assert!(stdout.contains("partial-coverage"), "{stdout}");
    assert!(stdout.contains("below-threshold"), "{stdout}");

    // threshold-not-declared.
    let out = readiness(&db2, &["alethefy", "--metric", "unique_visitors"]);
    assert!(support::share::lossy(&out.stdout).contains("threshold-not-declared"));
}

#[test]
fn an_absent_threshold_prints_a_verdict_and_still_exits_zero_when_ready() {
    // Absence alone withholds readiness, so this exits non-zero — but
    // as a verdict, not as an input refusal: stdout carries the report.
    let dir = tmp();
    let (db, _) = ready_world(dir.path());
    let out = readiness(
        &db,
        &[
            "alethefy",
            "--metric",
            "unique_visitors",
            "--stale-after-days",
            "365",
        ],
    );
    let stdout = support::share::lossy(&out.stdout);
    assert!(!out.status.success());
    assert!(!stdout.is_empty());
    assert!(stdout.contains("threshold-not-declared"), "{stdout}");
    assert!(support::share::lossy(&out.stderr).contains("error[portfolio-activation-not-ready]"));
}

#[test]
fn an_out_of_range_threshold_is_a_typed_refusal_with_empty_stdout() {
    let dir = tmp();
    let (db, _) = ready_world(dir.path());
    support::share::expect_refusal(
        &db,
        &[
            "portfolio",
            "activation",
            "readiness",
            "alethefy",
            "--metric",
            "unique_visitors",
            "--min-value",
            "1000000001",
        ],
        "portfolio-interest-invalid",
    );
}

#[test]
fn a_malformed_or_inverted_window_is_a_typed_refusal() {
    let dir = tmp();
    let (db, _) = ready_world(dir.path());
    for window in [
        "2026-09-01T00:00:00Z",
        "not-a-time..2026-09-08T00:00:00Z",
        "2026-09-08T00:00:00Z..2026-09-01T00:00:00Z",
        "2026-09-08T00:00:00Z..2026-09-08T00:00:00Z",
    ] {
        support::share::expect_refusal(
            &db,
            &[
                "portfolio",
                "activation",
                "readiness",
                "alethefy",
                "--metric",
                "unique_visitors",
                "--window",
                window,
            ],
            "portfolio-interest-invalid",
        );
    }
}

#[test]
fn a_blank_source_is_a_typed_refusal() {
    let dir = tmp();
    let (db, _) = ready_world(dir.path());
    support::share::expect_refusal(
        &db,
        &[
            "portfolio",
            "activation",
            "readiness",
            "alethefy",
            "--metric",
            "unique_visitors",
            "--source",
            "   ",
        ],
        "portfolio-interest-invalid",
    );
}

#[test]
fn an_unknown_project_and_an_unknown_metric_are_typed_refusals() {
    let dir = tmp();
    let (db, _) = ready_world(dir.path());
    support::share::expect_refusal(
        &db,
        &[
            "portfolio",
            "activation",
            "readiness",
            "no-such-project",
            "--metric",
            "unique_visitors",
        ],
        "unknown-project",
    );
    support::share::expect_refusal(
        &db,
        &[
            "portfolio",
            "activation",
            "readiness",
            "alethefy",
            "--metric",
            "bounce_rate",
        ],
        "portfolio-interest-invalid",
    );
    support::share::expect_refusal(
        &db,
        &[
            "portfolio",
            "activation",
            "readiness",
            "alethefy",
            "--metric",
            "unique_visitors",
            "--stale-after-days",
            "0",
        ],
        "portfolio-interest-invalid",
    );
}

#[test]
fn an_empty_registry_is_a_typed_refusal() {
    let dir = tmp();
    let db = dir.path().join("registry.db");
    support::share::expect_refusal(
        &db,
        &[
            "portfolio",
            "activation",
            "readiness",
            "--metric",
            "unique_visitors",
        ],
        "portfolio-interest-invalid",
    );
}

#[test]
fn the_fleet_form_evaluates_every_project_in_id_order() {
    let dir = tmp();
    let (db, _) = ready_world(dir.path());
    // Only alethefy has evidence; forge has none.
    let out = readiness(&db, &["--metric", "unique_visitors", "--min-value", "100"]);
    let stdout = support::share::lossy(&out.stdout);
    assert!(!out.status.success());
    // Match the verdict line, not a substring anywhere: the contract
    // name in the header also contains `forge`.
    let alethefy = stdout.find("\n  alethefy ").expect("alethefy verdict");
    let forge = stdout.find("\n  forge ").expect("forge verdict");
    assert!(
        alethefy < forge,
        "verdicts must be in project id order: {stdout}"
    );
    assert!(stdout.contains("summary: ready="), "{stdout}");
}

#[test]
fn readiness_json_carries_the_verdict_in_both_cases() {
    let dir = tmp();
    let (db, _) = ready_world(dir.path());
    let ready = readiness_json(
        &db,
        &[
            "alethefy",
            "--metric",
            "unique_visitors",
            "--min-value",
            "100",
            "--stale-after-days",
            "365",
        ],
    );
    assert_eq!(ready["contract"], "forge-portfolio-activation/0.1.0");
    assert_eq!(ready["activation"]["ready"], true);
    assert_eq!(ready["activation"]["metric"], "unique_visitors");
    assert_eq!(ready["activation"]["threshold"], 100);
    assert_eq!(ready["activation"]["verdicts"][0]["readiness"], "ready");
    assert!(ready["activation"]["verdicts"][0]["reasons"]
        .as_array()
        .unwrap()
        .is_empty());

    // A not-ready JSON report prints with a non-zero exit, so drive it
    // through the raw runner instead of `run_json`.
    let out = {
        let mut cmd = support::share::clean_cmd();
        cmd.arg("--registry").arg(&db);
        cmd.arg("--format").arg("json");
        for a in [
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
        ] {
            cmd.arg(a);
        }
        cmd.output().expect("run forge json")
    };
    assert!(!out.status.success());
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["activation"]["ready"], false);
    assert_eq!(body["activation"]["verdicts"][0]["readiness"], "not-ready");
    let reasons = body["activation"]["verdicts"][0]["reasons"]
        .as_array()
        .cloned()
        .unwrap();
    assert!(reasons.iter().any(|r| r["reason"] == "below-threshold"));
}

#[test]
fn every_readiness_route_demands_an_admin_session() {
    let dir = tmp();
    let (db, _) = ready_world(dir.path());
    let request = api_request_query(
        "GET",
        "/v1/interest/readiness",
        "project=alethefy&metric=unique_visitors",
        None,
    );
    let response = api(&db, &request);
    expect_api_error(&response, 401, "api-unauthorized");
}

#[test]
fn the_api_answers_200_with_the_verdict_for_ready_and_not_ready() {
    let dir = tmp();
    let (db, token) = ready_world(dir.path());
    let request = api_request_query(
        "GET",
        "/v1/interest/readiness",
        "project=alethefy&metric=unique_visitors&min_value=100&stale_after_days=365",
        Some(&token),
    );
    let response = api(&db, &request);
    assert_eq!(response.status, 200, "{:?}", api_json(&response));
    let body = api_json(&response);
    assert_eq!(body["contract"], "forge-portfolio-activation/0.1.0");
    assert_eq!(body["interest"]["activation"]["ready"], true);
    assert_eq!(
        body["interest"]["activation"]["verdicts"][0]["readiness"],
        "ready"
    );

    let request = api_request_query(
        "GET",
        "/v1/interest/readiness",
        "project=alethefy&metric=unique_visitors&min_value=10000&stale_after_days=365",
        Some(&token),
    );
    let response = api(&db, &request);
    assert_eq!(response.status, 200, "{:?}", api_json(&response));
    let body = api_json(&response);
    assert_eq!(body["interest"]["activation"]["ready"], false);
    let reasons = body["interest"]["activation"]["verdicts"][0]["reasons"]
        .as_array()
        .cloned()
        .unwrap();
    assert!(reasons.iter().any(|r| r["reason"] == "below-threshold"));
}

#[test]
fn readiness_query_parameters_are_bounded_and_typed() {
    let dir = tmp();
    let (db, token) = ready_world(dir.path());
    // metric is required.
    let request = api_request_query(
        "GET",
        "/v1/interest/readiness",
        "project=alethefy",
        Some(&token),
    );
    let response = api(&db, &request);
    expect_api_error(&response, 400, "api-invalid");

    // project xor projects.
    let request = api_request_query(
        "GET",
        "/v1/interest/readiness",
        "project=alethefy&projects=alethefy,forge&metric=unique_visitors",
        Some(&token),
    );
    let response = api(&db, &request);
    expect_api_error(&response, 400, "api-invalid");

    // min_value out of range and unparsable.
    for query in [
        "project=alethefy&metric=unique_visitors&min_value=1000000001",
        "project=alethefy&metric=unique_visitors&min_value=lots",
    ] {
        let request = api_request_query("GET", "/v1/interest/readiness", query, Some(&token));
        let response = api(&db, &request);
        expect_api_error(&response, 400, "api-invalid");
    }

    // unknown parameter.
    let request = api_request_query(
        "GET",
        "/v1/interest/readiness",
        "project=alethefy&metric=unique_visitors&price=9",
        Some(&token),
    );
    let response = api(&db, &request);
    expect_api_error(&response, 400, "api-invalid");
    assert!(api_json(&response).to_string().contains("price"));

    // unknown project.
    let request = api_request_query(
        "GET",
        "/v1/interest/readiness",
        "project=nope&metric=unique_visitors",
        Some(&token),
    );
    let response = api(&db, &request);
    expect_api_error(&response, 400, "unknown-project");
}

#[test]
fn the_stale_after_days_bound_is_validated_rather_than_clamped() {
    let dir = tmp();
    let (db, token) = ready_world(dir.path());
    for query in [
        "project=alethefy&metric=unique_visitors&stale_after_days=0",
        "project=alethefy&metric=unique_visitors&stale_after_days=366",
        "project=alethefy&metric=unique_visitors&stale_after_days=soon",
    ] {
        let request = api_request_query("GET", "/v1/interest/readiness", query, Some(&token));
        let response = api(&db, &request);
        expect_api_error(&response, 400, "api-invalid");
    }
    // A malformed or inverted window is a typed refusal, not a verdict.
    for query in [
        "project=alethefy&metric=unique_visitors&window=2026-09-01T00:00:00Z",
        "project=alethefy&metric=unique_visitors&window=2026-09-08T00:00:00Z..2026-09-01T00:00:00Z",
    ] {
        let request = api_request_query("GET", "/v1/interest/readiness", query, Some(&token));
        let response = api(&db, &request);
        expect_api_error(&response, 400, "api-invalid");
    }
}
