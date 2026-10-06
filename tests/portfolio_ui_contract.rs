//! Browser-UI contract for portfolio metadata and evidence
//! snapshots (`portfolio-metadata-and-review`).
//!
//! The portfolio package extends the existing in-process portal
//! UI rather than adding a second front end: `GET /ui` gains a
//! filter form and three portfolio columns, the project detail
//! page gains the portfolio projection and its write form, and
//! `POST /ui/projects/{id}/portfolio` is the only browser write
//! path. These tests drive the real HTTP surface and assert the
//! two ownership boundaries the change promises:
//!
//! - user-owned fields (lifecycle, confidence, tags, blocker,
//!   next action) are editable and filterable;
//! - imported evidence is attributed to its own source and
//!   revision, and an expired or unavailable observation reads
//!   `stale` / `unavailable`, never as a pass.
//!
//! The authorization tests re-use the bearer/`Origin`/form-token
//! checks the republish POST already established: an unauthorized
//! or cross-origin portfolio write persists no change.

use std::collections::BTreeMap;
use std::io::Cursor;
use std::path::PathBuf;

use chrono::{Duration, Utc};
use forge::api::{handle_buffered, ApiConfig, ApiRequest, ApiResponse};
use forge::registry::{Registry, SnapshotWrite};

const BEARER: &str = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
const LOOPBACK_ORIGIN: &str = "http://127.0.0.1:8765";

fn tmp_db_path() -> PathBuf {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut path = dir.path().to_path_buf();
    path.push("forge.db");
    let _ = dir;
    path
}

fn seed_project(path: &PathBuf, id: &str, profile: &str, maturity: &str) {
    {
        let _registry = Registry::open(path).expect("open registry");
    }
    let conn = rusqlite::Connection::open(path).expect("open");
    conn.execute(
        "INSERT INTO projects \
         (id, name, path, profile, maturity, target_maturity, \
          schema_version, platform_version, features, observed_at) \
         VALUES (?1, ?1, ?2, ?3, ?4, NULL, 1, '0.1.0', '{}', datetime('now')) \
         ON CONFLICT(id) DO UPDATE SET \
           name = excluded.name, \
           path = excluded.path, \
           profile = excluded.profile, \
           maturity = excluded.maturity",
        rusqlite::params![id, format!("/tmp/{id}"), profile, maturity],
    )
    .unwrap_or_else(|err| panic!("insert {id}: {err}"));
}

fn drive(config: &ApiConfig, db_path: &PathBuf, request: &ApiRequest) -> ApiResponse {
    let mut full: Vec<u8> = Vec::new();
    full.extend_from_slice(request.method.as_bytes());
    full.extend_from_slice(b" ");
    // The buffered transport re-parses the request line, so the
    // query string has to travel on the target, not only on the
    // pre-parsed `ApiRequest`.
    match request.query.as_deref() {
        Some(query) => {
            full.extend_from_slice(request.path.as_bytes());
            full.push(b'?');
            full.extend_from_slice(query.as_bytes());
        }
        None => full.extend_from_slice(request.path.as_bytes()),
    }
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

fn make_request(
    method: &str,
    path: &str,
    query: Option<&str>,
    headers: BTreeMap<String, String>,
    bearer: Option<String>,
    body: Vec<u8>,
) -> ApiRequest {
    let mut headers = headers;
    if let Some(token) = bearer.as_ref() {
        headers.insert("authorization".to_string(), format!("Bearer {token}"));
    }
    ApiRequest {
        method: method.to_string(),
        path: path.to_string(),
        query: query.map(|value| value.to_string()),
        headers,
        body,
        idempotency_key: None,
        bearer_token: bearer,
        cookies: BTreeMap::new(),
        remote_addr: Some("127.0.0.1:9999".parse().unwrap()),
        started_at: Utc::now(),
    }
}

fn html_accept() -> BTreeMap<String, String> {
    let mut h = BTreeMap::new();
    h.insert("accept".to_string(), "text/html".to_string());
    h
}

fn get(path: &str, query: Option<&str>) -> ApiResponse {
    let db = tmp_db_path();
    drive(
        &ApiConfig::default(),
        &db,
        &make_request(
            "GET",
            path,
            query,
            html_accept(),
            Some(BEARER.to_string()),
            Vec::new(),
        ),
    )
}

/// A GET against an already-seeded database.
fn get_from(db: &PathBuf, query: Option<&str>) -> ApiResponse {
    drive(
        &ApiConfig::default(),
        db,
        &make_request(
            "GET",
            "/ui",
            query,
            html_accept(),
            Some(BEARER.to_string()),
            Vec::new(),
        ),
    )
}

fn detail_from(db: &PathBuf, project_id: &str) -> ApiResponse {
    drive(
        &ApiConfig::default(),
        db,
        &make_request(
            "GET",
            &format!("/ui/projects/{project_id}"),
            None,
            html_accept(),
            Some(BEARER.to_string()),
            Vec::new(),
        ),
    )
}

fn form_headers(origin: Option<&str>) -> BTreeMap<String, String> {
    let mut headers = html_accept();
    headers.insert(
        "content-type".to_string(),
        "application/x-www-form-urlencoded".to_string(),
    );
    if let Some(origin) = origin {
        headers.insert("origin".to_string(), origin.to_string());
    }
    headers
}

fn post_portfolio(db: &PathBuf, project_id: &str, body: &str, origin: Option<&str>) -> ApiResponse {
    drive(
        &ApiConfig::default(),
        db,
        &make_request(
            "POST",
            &format!("/ui/projects/{project_id}/portfolio"),
            None,
            form_headers(origin),
            Some(BEARER.to_string()),
            body.as_bytes().to_vec(),
        ),
    )
}

// --- list filter -------------------------------------------------------

#[test]
fn list_renders_the_filter_form_and_the_portfolio_columns() {
    let db = tmp_db_path();
    seed_project(&db, "alethefy", "rust-web", "L3");
    let response = drive(
        &ApiConfig::default(),
        &db,
        &make_request(
            "GET",
            "/ui",
            None,
            html_accept(),
            Some(BEARER.to_string()),
            Vec::new(),
        ),
    );
    assert_eq!(response.status, 200);
    let body = String::from_utf8_lossy(&response.body);
    assert!(body.contains("Portfolio filter"), "{body}");
    assert!(body.contains("name=\"tag\""), "{body}");
    assert!(body.contains("name=\"lifecycle\""), "{body}");
    assert!(body.contains("name=\"confidence\""), "{body}");
    for vocabulary in ["incubating", "operational", "unknown", "high"] {
        assert!(body.contains(vocabulary), "missing option `{vocabulary}`");
    }
    for column in ["Lifecycle", "Confidence", "Tags", "Evidence"] {
        assert!(body.contains(column), "missing column `{column}`");
    }
}

#[test]
fn filter_by_tag_and_lifecycle_renders_only_matching_projects_with_evidence_states() {
    let db = tmp_db_path();
    seed_project(&db, "alethefy", "rust-web", "L3");
    seed_project(&db, "forge", "rust-web", "L2");
    seed_project(&db, "jenkins-local", "python-service", "L1");
    let registry = Registry::open(&db).expect("open");
    registry
        .portfolio_add_tag("alethefy", "platform", None)
        .expect("tag alethefy");
    registry
        .portfolio_add_tag("forge", "tooling", None)
        .expect("tag forge");
    registry
        .portfolio_write(
            "alethefy",
            &forge::registry::PortfolioWrite {
                lifecycle: Some(forge::portfolio::Lifecycle::Building),
                ..forge::registry::PortfolioWrite::default()
            },
        )
        .expect("lifecycle");
    registry
        .portfolio_write(
            "forge",
            &forge::registry::PortfolioWrite {
                lifecycle: Some(forge::portfolio::Lifecycle::Operational),
                ..forge::registry::PortfolioWrite::default()
            },
        )
        .expect("lifecycle");
    // An imported observation for the matching project only.
    registry
        .portfolio_import_snapshot(
            "alethefy",
            &SnapshotWrite {
                source_system: "gate".to_string(),
                source_revision: "a1b2c3".to_string(),
                observed_at: (Utc::now() - Duration::hours(1)).to_rfc3339(),
                status: forge::portfolio::EvidenceStatus::Observed,
                stale_after: Some((Utc::now() + Duration::hours(1)).to_rfc3339()),
                evidence_json: "{}".to_string(),
            },
        )
        .expect("snapshot");

    let response = get_from(&db, Some("tag=platform&lifecycle=building"));
    assert_eq!(response.status, 200);
    let body = String::from_utf8_lossy(&response.body);
    assert!(body.contains("alethefy"), "{body}");
    assert!(!body.contains("jenkins-local"), "{body}");
    assert!(
        !body.contains(">forge<"),
        "filtered-out project leaked: {body}"
    );
    assert!(body.contains("building"), "{body}");
    assert!(body.contains("platform"), "{body}");
    assert!(
        body.contains("gate"),
        "evidence source must be shown: {body}"
    );
    assert!(body.contains("observed"), "{body}");
    assert!(body.contains("Filtered by tag=platform"), "{body}");
}

#[test]
fn a_filter_that_matches_nothing_renders_an_explicit_empty_notice() {
    let db = tmp_db_path();
    seed_project(&db, "alethefy", "rust-web", "L3");
    let registry = Registry::open(&db).expect("open");
    registry
        .portfolio_add_tag("alethefy", "platform", None)
        .expect("tag");
    let response = get_from(&db, Some("lifecycle=archived"));
    assert_eq!(response.status, 200);
    let body = String::from_utf8_lossy(&response.body);
    assert!(
        body.contains("no project matches the active portfolio filter"),
        "{body}"
    );
    assert!(!body.contains("/ui/projects/alethefy"), "{body}");
    assert!(body.contains("Filtered by lifecycle=archived"), "{body}");
}

#[test]
fn an_out_of_vocabulary_filter_is_a_typed_refusal_not_a_widened_list() {
    let db = tmp_db_path();
    seed_project(&db, "alethefy", "rust-web", "L3");
    for query in [
        "lifecycle=shipped",
        "confidence=certain",
        "stage=alpha",
        "tag=Two%20Words",
    ] {
        let response = get_from(&db, Some(query));
        assert_eq!(response.status, 400, "{query}");
        let body = String::from_utf8_lossy(&response.body);
        assert!(body.contains("portfolio-invalid"), "{query}: {body}");
    }
}

#[test]
fn an_unclassified_project_reads_as_unclassified_not_as_a_default_lifecycle() {
    let db = tmp_db_path();
    seed_project(&db, "alethefy", "rust-web", "L3");
    let response = get_from(&db, None);
    assert_eq!(response.status, 200);
    let body = String::from_utf8_lossy(&response.body);
    assert!(body.contains("no evidence"), "{body}");
    // The lifecycle cell is the unclassified dash, not `incubating`.
    let row = body
        .split("<tr>")
        .find(|chunk| chunk.contains("alethefy"))
        .expect("project row");
    assert!(row.contains("row-skip"), "{row}");
    assert!(!row.contains("incubating"), "{row}");
}

// --- detail projection --------------------------------------------------

#[test]
fn detail_renders_the_portfolio_projection_for_an_unclassified_project() {
    let db = tmp_db_path();
    seed_project(&db, "alethefy", "rust-web", "L3");
    let response = detail_from(&db, "alethefy");
    assert_eq!(response.status, 200);
    let body = String::from_utf8_lossy(&response.body);
    assert!(body.contains("Portfolio"), "{body}");
    assert!(body.contains("Next action"), "{body}");
    assert!(body.contains("Blocker"), "{body}");
    assert!(body.contains("Edit portfolio metadata"), "{body}");
    assert!(
        body.contains("no source observation imported for this project"),
        "{body}"
    );
    assert!(
        body.contains("no review recorded for this project"),
        "{body}"
    );
}

#[test]
fn detail_attributes_every_evidence_row_to_its_source_and_revision() {
    let db = tmp_db_path();
    seed_project(&db, "alethefy", "rust-web", "L3");
    let registry = Registry::open(&db).expect("open");
    registry
        .portfolio_import_snapshot(
            "alethefy",
            &SnapshotWrite {
                source_system: "workspace-governance".to_string(),
                source_revision: "a1b2c3".to_string(),
                observed_at: (Utc::now() - Duration::hours(1)).to_rfc3339(),
                status: forge::portfolio::EvidenceStatus::Observed,
                stale_after: Some((Utc::now() + Duration::hours(1)).to_rfc3339()),
                evidence_json: "{}".to_string(),
            },
        )
        .expect("snapshot");

    let response = detail_from(&db, "alethefy");
    assert_eq!(response.status, 200);
    let body = String::from_utf8_lossy(&response.body);
    assert!(body.contains("workspace-governance"), "{body}");
    assert!(body.contains("a1b2c3"), "{body}");
    assert!(body.contains("observed"), "{body}");
    assert!(
        body.contains("Forge stores what the source reported; it does not run the check"),
        "{body}"
    );
}

#[test]
fn detail_never_renders_stale_or_unavailable_evidence_as_a_pass() {
    let db = tmp_db_path();
    seed_project(&db, "alethefy", "rust-web", "L3");
    let registry = Registry::open(&db).expect("open");
    registry
        .portfolio_import_snapshot(
            "alethefy",
            &SnapshotWrite {
                source_system: "governance".to_string(),
                source_revision: "old".to_string(),
                observed_at: (Utc::now() - Duration::days(30)).to_rfc3339(),
                status: forge::portfolio::EvidenceStatus::Observed,
                stale_after: Some((Utc::now() - Duration::days(1)).to_rfc3339()),
                evidence_json: "{}".to_string(),
            },
        )
        .expect("stale snapshot");
    registry
        .portfolio_import_snapshot(
            "alethefy",
            &SnapshotWrite {
                source_system: "runtime".to_string(),
                source_revision: "none".to_string(),
                observed_at: Utc::now().to_rfc3339(),
                status: forge::portfolio::EvidenceStatus::Unavailable,
                stale_after: None,
                evidence_json: "{}".to_string(),
            },
        )
        .expect("unavailable snapshot");

    let response = detail_from(&db, "alethefy");
    assert_eq!(response.status, 200);
    let body = String::from_utf8_lossy(&response.body);
    assert!(body.contains(">stale<"), "expected a stale cell: {body}");
    assert!(body.contains(">unavailable<"), "{body}");
    // No row in the evidence table carries the passing class.
    let table = body
        .split("<h4>Imported evidence</h4>")
        .nth(1)
        .and_then(|rest| rest.split("</table>").next())
        .expect("evidence table");
    assert!(!table.contains("row-ok"), "{table}");
}

#[test]
fn detail_renders_relations_in_both_directions_with_a_link_to_the_other_project() {
    let db = tmp_db_path();
    seed_project(&db, "alethefy", "rust-web", "L3");
    seed_project(&db, "forge", "rust-web", "L2");
    let registry = Registry::open(&db).expect("open");
    registry
        .portfolio_add_relation(
            "alethefy",
            "forge",
            forge::portfolio::RelationType::DependsOn,
            Some("shares the registry"),
        )
        .expect("relation");

    for (project, direction) in [("alethefy", "outgoing"), ("forge", "incoming")] {
        let response = detail_from(&db, project);
        assert_eq!(response.status, 200);
        let body = String::from_utf8_lossy(&response.body);
        assert!(body.contains("depends-on"), "{project}: {body}");
        assert!(body.contains(direction), "{project}: {body}");
        let other = if project == "alethefy" {
            "forge"
        } else {
            "alethefy"
        };
        assert!(
            body.contains(&format!("/ui/projects/{other}")),
            "{project}: {body}"
        );
        assert!(body.contains("shares the registry"), "{project}: {body}");
    }
}

#[test]
fn detail_renders_the_review_history() {
    let db = tmp_db_path();
    seed_project(&db, "alethefy", "rust-web", "L3");
    let registry = Registry::open(&db).expect("open");
    registry
        .portfolio_record_review(
            "alethefy",
            forge::portfolio::Confidence::High,
            Some("gate is green"),
        )
        .expect("review");
    let response = detail_from(&db, "alethefy");
    let body = String::from_utf8_lossy(&response.body);
    assert!(body.contains("gate is green"), "{body}");
    assert!(body.contains("high"), "{body}");
}

// --- browser write path -------------------------------------------------

#[test]
fn authorized_portfolio_post_persists_the_operator_metadata() {
    let db = tmp_db_path();
    seed_project(&db, "alethefy", "rust-web", "L3");
    let body = format!(
        "token={BEARER}&tag=platform&lifecycle=building&confidence=high\
         &next_action=cut+0.2.0&blocker=waiting+on+the+mac"
    );
    let response = post_portfolio(&db, "alethefy", &body, Some(LOOPBACK_ORIGIN));
    assert_eq!(
        response.status,
        200,
        "{}",
        String::from_utf8_lossy(&response.body)
    );
    let text = String::from_utf8_lossy(&response.body);
    assert!(text.contains("Saved portfolio metadata"), "{text}");
    assert!(text.contains("tagged with platform"), "{text}");
    assert!(
        response
            .headers
            .iter()
            .any(|(k, v)| k == "location" && v == "/ui/projects/alethefy"),
        "expected a Location header: {:?}",
        response.headers
    );

    let detail = String::from_utf8_lossy(&detail_from(&db, "alethefy").body).to_string();
    assert!(detail.contains("building"), "{detail}");
    assert!(detail.contains("high"), "{detail}");
    assert!(detail.contains("cut 0.2.0"), "{detail}");
    assert!(detail.contains("waiting on the mac"), "{detail}");
    assert!(detail.contains("platform"), "{detail}");
}

#[test]
fn a_portfolio_post_without_a_bearer_token_persists_no_change() {
    let db = tmp_db_path();
    seed_project(&db, "alethefy", "rust-web", "L3");
    let body = format!("token={BEARER}&tag=platform&lifecycle=building");
    let response = drive(
        &ApiConfig::default(),
        &db,
        &make_request(
            "POST",
            "/ui/projects/alethefy/portfolio",
            None,
            form_headers(Some(LOOPBACK_ORIGIN)),
            None,
            body.into_bytes(),
        ),
    );
    assert_eq!(response.status, 401);
    let text = String::from_utf8_lossy(&response.body);
    assert!(text.contains("api-unauthorized"), "{text}");
    assert_no_portfolio_state(&db);
}

#[test]
fn a_cross_origin_portfolio_post_persists_no_change() {
    let db = tmp_db_path();
    seed_project(&db, "alethefy", "rust-web", "L3");
    let body = format!("token={BEARER}&tag=platform&lifecycle=building");
    let response = post_portfolio(&db, "alethefy", &body, Some("http://evil.example.com"));
    assert_eq!(response.status, 403);
    let text = String::from_utf8_lossy(&response.body);
    assert!(text.contains("ui-origin-mismatch"), "{text}");
    assert_no_portfolio_state(&db);
}

#[test]
fn a_portfolio_post_with_a_mismatched_form_token_persists_no_change() {
    let db = tmp_db_path();
    seed_project(&db, "alethefy", "rust-web", "L3");
    let body = "token=not_the_bearer&tag=platform&lifecycle=building";
    let response = post_portfolio(&db, "alethefy", body, Some(LOOPBACK_ORIGIN));
    assert_eq!(response.status, 403);
    let text = String::from_utf8_lossy(&response.body);
    assert!(text.contains("api-token-mismatch"), "{text}");
    assert_no_portfolio_state(&db);
}

#[test]
fn an_invalid_portfolio_field_is_a_typed_refusal_and_writes_nothing() {
    let db = tmp_db_path();
    seed_project(&db, "alethefy", "rust-web", "L3");
    for body in [
        format!("token={BEARER}&lifecycle=shipped"),
        format!("token={BEARER}&confidence=certain"),
        format!("token={BEARER}&tag=Two+Words"),
        // A control character in a free-text field is refused
        // before it reaches the table cell.
        format!("token={BEARER}&next_action=line%0Abreak"),
        // No editable field at all.
        format!("token={BEARER}"),
    ] {
        let response = post_portfolio(&db, "alethefy", &body, Some(LOOPBACK_ORIGIN));
        assert_eq!(response.status, 400, "{body}");
        let text = String::from_utf8_lossy(&response.body);
        assert!(text.contains("portfolio-invalid"), "{body}: {text}");
        assert_no_portfolio_state(&db);
    }
}

#[test]
fn a_portfolio_post_for_an_unknown_project_is_a_typed_404() {
    let db = tmp_db_path();
    seed_project(&db, "alethefy", "rust-web", "L3");
    let body = format!("token={BEARER}&tag=platform");
    let response = post_portfolio(&db, "no-such-app", &body, Some(LOOPBACK_ORIGIN));
    assert_eq!(response.status, 404);
    let text = String::from_utf8_lossy(&response.body);
    assert!(text.contains("unknown-project"), "{text}");
    assert_no_portfolio_state(&db);
}

#[test]
fn a_browser_post_never_writes_an_evidence_snapshot() {
    let db = tmp_db_path();
    seed_project(&db, "alethefy", "rust-web", "L3");
    // Even a form that tries to smuggle an evidence field through
    // an unknown key is refused or ignored; imported evidence has
    // no browser write path.
    let body = format!(
        "token={BEARER}&tag=platform&source=gate&revision=r1&status=observed&evidence=%7B%7D"
    );
    let response = post_portfolio(&db, "alethefy", &body, Some(LOOPBACK_ORIGIN));
    assert_eq!(response.status, 200);
    let registry = Registry::open(&db).expect("open");
    assert!(registry
        .portfolio_snapshots_for("alethefy", 10)
        .expect("snapshots")
        .is_empty());
    // And the form carries no evidence input.
    let detail = String::from_utf8_lossy(&detail_from(&db, "alethefy").body).to_string();
    assert!(detail.contains("Edit portfolio metadata"), "{detail}");
    assert!(!detail.contains("name=\"source\""), "{detail}");
}

#[test]
fn removing_a_tag_through_the_browser_form_is_idempotent() {
    let db = tmp_db_path();
    seed_project(&db, "alethefy", "rust-web", "L3");
    let registry = Registry::open(&db).expect("open");
    registry
        .portfolio_add_tag("alethefy", "platform", None)
        .expect("tag");

    let body = format!("token={BEARER}&remove_tag=platform");
    let response = post_portfolio(&db, "alethefy", &body, Some(LOOPBACK_ORIGIN));
    assert_eq!(response.status, 200);
    let text = String::from_utf8_lossy(&response.body);
    assert!(text.contains("removed tag platform"), "{text}");

    let response = post_portfolio(&db, "alethefy", &body, Some(LOOPBACK_ORIGIN));
    assert_eq!(response.status, 200);
    let text = String::from_utf8_lossy(&response.body);
    assert!(text.contains("nothing changed"), "{text}");
    let registry = Registry::open(&db).expect("open");
    assert!(registry
        .portfolio_tags_for("alethefy")
        .expect("tags")
        .is_empty());
}

/// A portfolio write is journaled so the operator's edit is
/// auditable; it must not create a `publish.ui` row or touch any
/// repository file.
#[test]
fn a_browser_portfolio_write_journals_one_portfolio_ui_row() {
    let db = tmp_db_path();
    seed_project(&db, "alethefy", "rust-web", "L3");
    let body = format!("token={BEARER}&tag=platform");
    let response = post_portfolio(&db, "alethefy", &body, Some(LOOPBACK_ORIGIN));
    assert_eq!(response.status, 200);
    let registry = Registry::open(&db).expect("open");
    let recent = registry.recent_operations(64).expect("recent");
    assert_eq!(
        recent
            .iter()
            .filter(|e| e.kind == "portfolio.ui" && e.project_id == "alethefy")
            .count(),
        1
    );
    assert!(!recent.iter().any(|e| e.kind == "publish.ui"));
}

/// Assert no portfolio rows landed. Used after every refused
/// write so a refusal is proven inert, not merely refused.
fn assert_no_portfolio_state(db: &PathBuf) {
    let registry = Registry::open(db).expect("open");
    assert!(
        registry
            .portfolio_tags_for("alethefy")
            .expect("tags")
            .is_empty(),
        "a refused write left a tag behind"
    );
    assert!(
        registry
            .portfolio_record("alethefy")
            .expect("record")
            .lifecycle
            .is_none(),
        "a refused write left a lifecycle behind"
    );
    assert!(
        registry
            .portfolio_reviews_for("alethefy", 10)
            .expect("reviews")
            .is_empty(),
        "a refused write left a review behind"
    );
    assert!(
        registry
            .portfolio_snapshots_for("alethefy", 10)
            .expect("snapshots")
            .is_empty(),
        "a refused write left an evidence snapshot behind"
    );
}

// --- compatibility ------------------------------------------------------

#[test]
fn the_existing_ui_routes_still_answer_without_portfolio_state() {
    let db = tmp_db_path();
    seed_project(&db, "alethefy", "rust-web", "L3");

    let list = drive(
        &ApiConfig::default(),
        &db,
        &make_request(
            "GET",
            "/ui",
            None,
            html_accept(),
            Some(BEARER.to_string()),
            Vec::new(),
        ),
    );
    assert_eq!(list.status, 200);
    assert!(String::from_utf8_lossy(&list.body).contains("fleet online"));

    let detail = detail_from(&db, "alethefy");
    assert_eq!(detail.status, 200);
    let text = String::from_utf8_lossy(&detail.body);
    assert!(text.contains("Republish"));
    assert!(text.contains("Recent operations"));

    // The publish plan preview still journals nothing.
    let mut headers = form_headers(Some(LOOPBACK_ORIGIN));
    headers.insert(
        "content-type".to_string(),
        "application/x-www-form-urlencoded".to_string(),
    );
    let plan = drive(
        &ApiConfig::default(),
        &db,
        &make_request(
            "POST",
            "/ui/projects/alethefy/publish",
            None,
            headers,
            Some(BEARER.to_string()),
            format!("token={BEARER}").into_bytes(),
        ),
    );
    assert_eq!(plan.status, 200);
    let registry = Registry::open(&db).expect("open");
    assert!(registry
        .recent_operations(64)
        .expect("recent")
        .iter()
        .all(|e| e.kind != "publish.ui"));
}
