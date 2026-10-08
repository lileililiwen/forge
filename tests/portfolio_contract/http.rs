//! HTTP contract through the authorization boundary plus the registry-level portal projection.

use super::*;

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
