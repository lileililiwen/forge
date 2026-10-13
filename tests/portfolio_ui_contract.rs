//! Portfolio metadata and evidence contract (`portfolio-metadata-and-review`)
//! driven against the surface that actually ships today.
//!
//! `7d2df41` ("remove the in-process /ui HTML portal") deleted
//! `src/api/ui/`, and with it `GET /ui`, `GET /ui/projects/{id}` and
//! `POST /ui/projects/{id}/portfolio`. The capability did not go with
//! them: the JSON API the SPA calls is
//! `forge-web-portfolio-controls/0.1.0` under `/v1/admin/portfolio`,
//! and `frontend/index.html:434` renders the `Lifecycle | Confidence |
//! Tags | Evidence` table. Every test below therefore drives the real
//! HTTP dispatcher (`forge::api::handle`) against the surviving route,
//! and asserts only what a canonical spec already declares:
//!
//! - `portfolio-metadata-and-review` — "Filter by tag and lifecycle",
//!   "Fresh evidence", "Stale or unavailable provider", "Valid
//!   dependency relation", "Tag and review project", "Unknown project",
//!   "Unauthorized mutation", and "Project portfolio data through the
//!   portal" (detail fields).
//! - `forge-web-portfolio-controls` — "Attempt to edit imported
//!   evidence", "Record portfolio review".
//! - `web-portfolio-completion` — confirm-gated writes, idempotent
//!   `tags/remove`, sub-resource read-back, unknown read kind 404.
//! - `portal-spa-convergence` — the SPA is the single interactive
//!   surface; no second `/ui` portal answers.
//!
//! The three authorization tests re-use the checks the surviving admin
//! gate actually runs, in the order it runs them
//! (`src/api/admin/deploy.rs:65` origin, then the JSON content type,
//! then `gateway.rs:456` the session gate): a cross-origin, a
//! form-encoded and an unauthenticated portfolio write each persist no
//! change.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{Duration, Utc};
use forge::api::{handle, ApiConfig, ApiRequest, ApiResponse};
use forge::registry::{Registry, SnapshotWrite};
use serde_json::Value;
use tempfile::TempDir;

const OPERATOR_EMAIL: &str = "operator@example.test";
const OPERATOR_PASSWORD: &str = "a-long-test-password";

/// A registered registry plus the administrator session the browser
/// holds in its `forge_admin_session` cookie.
struct Fixture {
    dir: TempDir,
    db: PathBuf,
    config: ApiConfig,
    token: String,
}

impl Fixture {
    fn new(projects: &[(&str, &str)]) -> Fixture {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = dir.path().join("registry.db");
        let config = ApiConfig::default();
        // A real project directory per seeded id, so a portfolio write
        // that touched a repository file would be observable.
        let mut registry = Registry::open(&db).expect("open registry");
        for (id, profile) in projects {
            let proj = dir.path().join(format!("src/{id}"));
            fs::create_dir_all(&proj).expect("project dir");
            fs::write(
                proj.join("forge.yaml"),
                format!(
                    "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: {profile}\n  maturity: L1\nruntime:\n  language: rust\n"
                ),
            )
            .expect("manifest");
            fs::write(proj.join("README.md"), format!("# {id}\n")).expect("readme");
            registry.register(&proj, None).expect("register");
        }
        drop(registry);

        forge::identity::global::setup(&db, OPERATOR_EMAIL, OPERATOR_PASSWORD)
            .expect("configure the Forge administrator");
        let token = forge::identity::global::authenticate(&db, OPERATOR_EMAIL, OPERATOR_PASSWORD)
            .expect("authenticate")
            .expect("a session token");
        Fixture {
            dir,
            db,
            config,
            token,
        }
    }

    /// The browser's own request shape: the configured frontend origin,
    /// the HttpOnly session cookie and, for a mutation, a JSON body.
    fn get(&self, path: &str, query: Option<&str>) -> ApiResponse {
        self.request(
            "GET",
            path,
            query,
            Some(&self.token),
            Some("application/json"),
            b"",
        )
    }

    fn post_json(&self, path: &str, body: &[u8]) -> ApiResponse {
        self.request(
            "POST",
            path,
            None,
            Some(&self.token),
            Some("application/json"),
            body,
        )
    }

    fn request(
        &self,
        method: &str,
        path: &str,
        query: Option<&str>,
        cookie: Option<&str>,
        content_type: Option<&str>,
        body: &[u8],
    ) -> ApiResponse {
        let mut headers =
            BTreeMap::from([("origin".to_string(), self.config.frontend_origin.clone())]);
        if let Some(content_type) = content_type {
            headers.insert("content-type".to_string(), content_type.to_string());
        }
        let cookies = cookie
            .map(|token| BTreeMap::from([("forge_admin_session".to_string(), token.to_string())]))
            .unwrap_or_default();
        handle(
            &self.config,
            &self.db,
            &ApiRequest {
                method: method.to_string(),
                path: path.to_string(),
                query: query.map(|value| value.to_string()),
                headers,
                body: body.to_vec(),
                idempotency_key: None,
                bearer_token: None,
                cookies,
                remote_addr: Some("127.0.0.1:9999".parse().unwrap()),
                started_at: Utc::now(),
            },
            Utc::now(),
        )
    }
}

fn body_json(response: &ApiResponse) -> Value {
    serde_json::from_slice(&response.body).unwrap_or_else(|err| {
        panic!(
            "expected JSON, got {}: {err}",
            String::from_utf8_lossy(&response.body)
        )
    })
}

fn error_code(response: &ApiResponse) -> String {
    body_json(response)["error"]["code"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

fn project_row<'a>(list: &'a Value, project_id: &str) -> &'a Value {
    list["projects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["project_id"] == project_id)
        .unwrap_or_else(|| panic!("`{project_id}` missing from the fleet list: {list}"))
}

fn registry(db: &Path) -> Registry {
    Registry::open(db).expect("open registry")
}

fn write_project_file(fixture: &Fixture, id: &str) -> PathBuf {
    fixture.dir.path().join(format!("src/{id}/README.md"))
}

fn frontend_file(name: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(root.join("frontend").join(name))
        .unwrap_or_else(|err| panic!("read frontend/{name}: {err}"))
}

// --- list: filters and the portfolio columns ------------------------------

/// The `/ui` list rendered a filter form plus four portfolio columns.
/// The filter vocabulary moved to the typed query of
/// `GET /v1/admin/portfolio` (`src/api/portfolio.rs:545`) and the
/// columns are rendered by the SPA table over the same row fields, so
/// this is split: the API half proves the filter keys and the row
/// fields exist; the frontend half proves the shipped table carries the
/// four column headers. Chosen over a pure frontend assertion because
/// the filter capability lives on the route, and over a pure API
/// assertion because the columns live in `frontend/`.
#[test]
fn list_renders_the_filter_form_and_the_portfolio_columns() {
    let fixture = Fixture::new(&[("alethefy", "rust-web")]);
    let response = fixture.get("/v1/admin/portfolio", None);
    assert_eq!(response.status, 200);
    let list = body_json(&response);

    // The typed filter vocabulary the operator may narrow by.
    let source =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/api/portfolio.rs"))
            .expect("read the portfolio route module");
    for filter in ["\"tag\"", "\"lifecycle\"", "\"confidence\""] {
        assert!(
            source.contains(filter),
            "`GET /v1/admin/portfolio` no longer offers the `{filter}` filter"
        );
    }

    // The row carries the four columns the portal renders, plus the
    // echoed filter so the operator sees what was applied.
    let row = project_row(&list, "alethefy");
    for column in ["lifecycle", "confidence", "tags", "evidence"] {
        assert!(row.get(column).is_some(), "missing row column `{column}`");
    }
    assert!(list["filter"].is_object(), "the applied filter is echoed");
    assert_eq!(list["filter"]["tag"], Value::Null);

    // The shipped SPA table renders those columns. The portfolio table is
    // identified by its row container, so the projects table's header
    // cannot satisfy this.
    let index = frontend_file("index.html");
    let rows_at = index
        .find("<tbody id=\"portfolio-rows\">")
        .expect("the portfolio row container");
    let header = index[..rows_at]
        .rsplit_once("<thead>")
        .map(|(_, before)| before)
        .expect("the portfolio table header");
    for column in ["Lifecycle", "Confidence", "Tags", "Evidence"] {
        assert!(
            header.contains(&format!(">{column}<")),
            "the portfolio table lost its `{column}` column: {header}"
        );
    }
    let app = frontend_file("app.js");
    let render = app
        .find("function renderPortfolioProjects")
        .expect("the portfolio row renderer");
    let render = &app[render
        ..app[render..]
            .find("function populatePortfolioProjects")
            .unwrap()
            + render];
    for field in [
        "project.lifecycle",
        "project.confidence",
        "project.tags",
        "project.evidence_summary",
    ] {
        assert!(
            render.contains(field),
            "the row renderer no longer binds `{field}`"
        );
    }
}

#[test]
fn filter_by_tag_and_lifecycle_renders_only_matching_projects_with_evidence_states() {
    let fixture = Fixture::new(&[
        ("alethefy", "rust-web"),
        ("forge", "rust-web"),
        ("jenkins-local", "python-service"),
    ]);
    {
        let registry = registry(&fixture.db);
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
    }

    let response = fixture.get(
        "/v1/admin/portfolio",
        Some("tag=platform&lifecycle=building"),
    );
    assert_eq!(response.status, 200);
    let list = body_json(&response);

    // Only the project matching both keys is returned.
    let ids: Vec<&str> = list["projects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["project_id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec!["alethefy"], "{list}");
    assert_eq!(list["summary"]["projects"], 1);

    let row = project_row(&list, "alethefy");
    assert_eq!(row["lifecycle"], "building");
    assert_eq!(
        row["tags"],
        serde_json::json!(["platform"]),
        "the matching tag is shown"
    );
    // The evidence state is rendered with its own source.
    let evidence = row["evidence"].as_array().expect("evidence rows");
    assert_eq!(evidence.len(), 1, "{row}");
    assert_eq!(evidence[0]["source_system"], "gate");
    assert_eq!(evidence[0]["source_revision"], "a1b2c3");
    assert_eq!(evidence[0]["status"], "observed");
    assert_eq!(row["evidence_summary"], "gate observed");

    // The applied filter is echoed so the operator sees what narrowed it.
    assert_eq!(list["filter"]["tag"], "platform");
    assert_eq!(list["filter"]["lifecycle"], "building");
}

/// A filter that matches nothing is an honest empty list with the filter
/// still echoed — the JSON half of the old "explicit empty notice". The
/// notice itself is the SPA's `portfolio-project-empty` element, so this
/// asserts both halves.
#[test]
fn a_filter_that_matches_nothing_renders_an_explicit_empty_notice() {
    let fixture = Fixture::new(&[("alethefy", "rust-web")]);
    registry(&fixture.db)
        .portfolio_add_tag("alethefy", "platform", None)
        .expect("tag");

    let response = fixture.get("/v1/admin/portfolio", Some("lifecycle=archived"));
    assert_eq!(response.status, 200);
    let list = body_json(&response);
    assert_eq!(list["projects"], serde_json::json!([]), "{list}");
    assert_eq!(list["summary"]["projects"], 0);
    assert_eq!(list["summary"]["with_evidence"], 0);
    // The filter is still named, so an empty list is never mistaken for
    // an unfiltered one.
    assert_eq!(list["filter"]["lifecycle"], "archived");

    // The SPA shows its own empty state exactly when the list is empty.
    let app = frontend_file("app.js");
    assert!(
        app.contains(
            "document.getElementById(\"portfolio-project-empty\").hidden = projects.length > 0;"
        ),
        "the empty notice is no longer driven by the list length"
    );
    let index = frontend_file("index.html");
    assert!(
        index.contains("id=\"portfolio-project-empty\""),
        "the explicit empty notice element is gone"
    );
    // And no project row is rendered for a filtered-out project.
    assert!(!index.contains("/ui/projects/alethefy"));
}

/// An out-of-vocabulary filter is a typed refusal, never a widened list.
/// The closed vocabulary is `Lifecycle`/`Confidence` plus the three
/// known filter keys; a filter tag is a free label and is validated at
/// the write boundary instead (see the invalid-field test).
#[test]
fn an_out_of_vocabulary_filter_is_a_typed_refusal_not_a_widened_list() {
    let fixture = Fixture::new(&[("alethefy", "rust-web")]);
    for query in ["lifecycle=shipped", "confidence=certain", "stage=alpha"] {
        let response = fixture.get("/v1/admin/portfolio", Some(query));
        assert_eq!(response.status, 400, "{query}");
        assert_eq!(error_code(&response), "portfolio-invalid", "{query}");
        let body = body_json(&response);
        assert_eq!(body["effect"], "none", "{query}");
        assert!(
            body.get("projects").is_none(),
            "a refused filter returned a list: {body}"
        );
    }
}

/// An unclassified project is absent, never defaulted. The list row
/// carries `null` lifecycle/confidence, and the closed filter contract
/// (`PortfolioFilter::matches`) refuses to match an absent field.
#[test]
fn an_unclassified_project_reads_as_unclassified_not_as_a_default_lifecycle() {
    let fixture = Fixture::new(&[("alethefy", "rust-web")]);

    let list = body_json(&fixture.get("/v1/admin/portfolio", None));
    let row = project_row(&list, "alethefy");
    assert_eq!(row["lifecycle"], Value::Null, "{row}");
    assert_eq!(row["confidence"], Value::Null, "{row}");
    assert_eq!(row["evidence_summary"], "no evidence", "{row}");

    // Asking for a lifecycle it does not have returns nothing rather
    // than reporting it as the first lifecycle in the vocabulary.
    let filtered = body_json(&fixture.get("/v1/admin/portfolio", Some("lifecycle=incubating")));
    assert_eq!(filtered["projects"], serde_json::json!([]), "{filtered}");

    // And the detail projection agrees.
    let detail = body_json(&fixture.get("/v1/admin/portfolio/alethefy", None));
    assert_eq!(detail["portfolio"]["profile"]["lifecycle"], Value::Null);
}

// --- detail projection -----------------------------------------------------

#[test]
fn detail_renders_the_portfolio_projection_for_an_unclassified_project() {
    let fixture = Fixture::new(&[("alethefy", "rust-web")]);
    let response = fixture.get("/v1/admin/portfolio/alethefy", None);
    assert_eq!(response.status, 200);
    let detail = body_json(&response);
    assert_eq!(detail["contract"], "forge-web-portfolio-controls/0.1.0");
    let portfolio = &detail["portfolio"];
    let profile = &portfolio["profile"];
    assert_eq!(profile["project_id"], "alethefy");
    assert_eq!(profile["lifecycle"], Value::Null, "{profile}");
    assert_eq!(profile["confidence"], Value::Null, "{profile}");
    assert_eq!(profile["next_action"], Value::Null, "{profile}");
    assert_eq!(profile["blocker"], Value::Null, "{profile}");
    assert_eq!(profile["reviewed_at"], Value::Null, "{profile}");
    for empty in ["tags", "goals", "relations", "evidence", "reviews"] {
        assert_eq!(
            portfolio[empty],
            serde_json::json!([]),
            "`{empty}` should be empty for an unclassified project: {portfolio}"
        );
    }

    // The portal renders those detail fields: the SPA's single-project
    // reader is the surviving detail surface.
    let app = frontend_file("app.js");
    let start = app
        .find("async function portfolioShow")
        .expect("the portfolio show reader");
    let reader = &app[start..start + 2500];
    for field in [
        "detailRow(\"Lifecycle\"",
        "detailRow(\"Confidence\"",
        "detailRow(\"Next action\"",
        "detailRow(\"Blocker\"",
        "detailRow(\"Tags\"",
        "detailRow(\"Evidence\"",
    ] {
        assert!(reader.contains(field), "the detail reader lost `{field}`");
    }
    let index = frontend_file("index.html");
    assert!(index.contains("id=\"portfolio-show\""));
    assert!(index.contains("id=\"portfolio-show-result\""));
}

/// Every imported observation is attributed to its own source and
/// revision and is marked non-editable, so the read model never claims
/// Forge performed the check and never offers a browser edit.
#[test]
fn detail_attributes_every_evidence_row_to_its_source_and_revision() {
    let fixture = Fixture::new(&[("alethefy", "rust-web")]);
    registry(&fixture.db)
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

    let detail = body_json(&fixture.get("/v1/admin/portfolio/alethefy", None));
    let rows = detail["portfolio"]["evidence"]
        .as_array()
        .expect("evidence");
    assert_eq!(rows.len(), 1, "{detail}");
    assert_eq!(rows[0]["source_system"], "workspace-governance");
    assert_eq!(rows[0]["source_revision"], "a1b2c3");
    assert_eq!(rows[0]["status"], "observed");
    assert!(rows[0]["observed_at"].is_string());

    // The dedicated evidence read carries provenance and provenance's
    // ownership: the row is attributed and not editable.
    let evidence = body_json(&fixture.get("/v1/admin/portfolio/alethefy/evidence", None));
    let items = evidence["evidence"].as_array().expect("evidence list");
    assert_eq!(items.len(), 1, "{evidence}");
    assert_eq!(items[0]["source_system"], "workspace-governance");
    assert_eq!(items[0]["source_revision"], "a1b2c3");
    assert_eq!(items[0]["status"], "observed");
    assert_eq!(
        items[0]["editable"], false,
        "imported evidence must be read-only in the browser"
    );
}

#[test]
fn detail_never_renders_stale_or_unavailable_evidence_as_a_pass() {
    let fixture = Fixture::new(&[("alethefy", "rust-web")]);
    {
        let registry = registry(&fixture.db);
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
    }

    let evidence = body_json(&fixture.get("/v1/admin/portfolio/alethefy/evidence", None));
    let rows = evidence["evidence"].as_array().expect("evidence");
    assert_eq!(rows.len(), 2, "{evidence}");
    let by_source: BTreeMap<&str, &Value> = rows
        .iter()
        .map(|row| (row["source_system"].as_str().unwrap(), row))
        .collect();
    // The expired observation displays `stale`, the unreachable source
    // displays `unavailable`; neither is upgraded to a reporting pass.
    assert_eq!(by_source["governance"]["status"], "stale");
    assert_eq!(by_source["runtime"]["status"], "unavailable");
    for source in ["governance", "runtime"] {
        assert_ne!(
            by_source[source]["status"], "observed",
            "`{source}` must not read as a fresh observation"
        );
        assert_ne!(by_source[source]["status"], "healthy");
        assert_eq!(by_source[source]["editable"], false);
    }

    // The fleet row and the detail projection agree: neither reports the
    // expired observation as a pass, and the recorded state is preserved
    // verbatim beside the effective one.
    let list = body_json(&fixture.get("/v1/admin/portfolio", None));
    let row = project_row(&list, "alethefy");
    assert_eq!(
        row["evidence_summary"],
        "governance stale, runtime unavailable"
    );
    let detail = body_json(&fixture.get("/v1/admin/portfolio/alethefy", None));
    let detail_rows = detail["portfolio"]["evidence"]
        .as_array()
        .expect("evidence");
    let by_source: BTreeMap<&str, &Value> = detail_rows
        .iter()
        .map(|row| (row["source_system"].as_str().unwrap(), row))
        .collect();
    assert_eq!(
        by_source["governance"]["effective_status"], "stale",
        "an expired observation is displayed as a pass: {detail_rows:?}"
    );
    assert_eq!(
        by_source["governance"]["status"], "observed",
        "the recorded source state must be preserved verbatim: {detail_rows:?}"
    );
    assert_eq!(by_source["runtime"]["effective_status"], "unavailable");
    assert_eq!(by_source["runtime"]["status"], "unavailable");
}

#[test]
fn detail_renders_relations_in_both_directions_with_a_link_to_the_other_project() {
    let fixture = Fixture::new(&[("alethefy", "rust-web"), ("forge", "rust-web")]);
    registry(&fixture.db)
        .portfolio_add_relation(
            "alethefy",
            "forge",
            forge::portfolio::RelationType::DependsOn,
            Some("shares the registry"),
        )
        .expect("relation");

    for (project, direction, other) in [
        ("alethefy", "outgoing", "forge"),
        ("forge", "incoming", "alethefy"),
    ] {
        let response = fixture.get(&format!("/v1/admin/portfolio/{project}/relations"), None);
        assert_eq!(response.status, 200);
        let body = body_json(&response);
        let rows = body["relations"].as_array().expect("relations");
        assert_eq!(rows.len(), 1, "{project}: {body}");
        assert_eq!(rows[0]["direction"], direction, "{project}: {body}");
        assert_eq!(rows[0]["relation_type"], "depends-on", "{project}: {body}");
        // The other project is named, so the reader can reach it.
        assert_eq!(rows[0]["other_project"], other, "{project}: {body}");
        assert_eq!(rows[0]["note"], "shares the registry", "{project}: {body}");

        // The single-project projection carries the same relation.
        let detail = body_json(&fixture.get(&format!("/v1/admin/portfolio/{project}"), None));
        let relations = detail["portfolio"]["relations"]
            .as_array()
            .expect("relations");
        assert_eq!(relations.len(), 1, "{project}: {detail}");
        assert_eq!(relations[0]["other_project"], other);

        // And the SPA detail reader names the direction and the other
        // project, which is what replaced the old in-page link.
        let app = frontend_file("app.js");
        assert!(
            app.contains("detailRow(\"Relations\", (portfolio.relations || [])"),
            "the detail reader no longer renders relations"
        );
    }
}

#[test]
fn detail_renders_the_review_history() {
    let fixture = Fixture::new(&[("alethefy", "rust-web")]);
    registry(&fixture.db)
        .portfolio_record_review(
            "alethefy",
            forge::portfolio::Confidence::High,
            Some("gate is green"),
        )
        .expect("review");

    let response = fixture.get("/v1/admin/portfolio/alethefy/reviews", None);
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    let reviews = body["reviews"].as_array().expect("reviews");
    assert_eq!(reviews.len(), 1, "{body}");
    assert_eq!(reviews[0]["confidence"], "high");
    assert_eq!(reviews[0]["note"], "gate is green");
    assert_eq!(reviews[0]["project_id"], "alethefy");
    assert!(reviews[0]["reviewed_at"].is_string());

    // The detail projection counts and exposes the same history.
    let detail = body_json(&fixture.get("/v1/admin/portfolio/alethefy", None));
    assert_eq!(
        detail["portfolio"]["reviews"]
            .as_array()
            .expect("reviews")
            .len(),
        1
    );

    // The SPA renders the review history list.
    let index = frontend_file("index.html");
    assert!(index.contains("id=\"portfolio-review-list\""));
}

// --- browser write path ----------------------------------------------------

/// An authorized, confirmed portfolio write persists the operator's
/// user-owned metadata and reads it back through the projection. The
/// confirm gate is part of the surviving contract
/// (`web-portfolio-completion`), so both writes carry `confirm: true`.
#[test]
fn authorized_portfolio_post_persists_the_operator_metadata() {
    let fixture = Fixture::new(&[("alethefy", "rust-web")]);

    let tag = fixture.post_json(
        "/v1/admin/portfolio/alethefy/tags",
        br#"{"name":"platform","confirm":true}"#,
    );
    assert_eq!(tag.status, 200, "{}", String::from_utf8_lossy(&tag.body));
    let tag_body = body_json(&tag);
    assert_eq!(tag_body["effect"], "forge-owned-write");
    assert_eq!(tag_body["result"]["tag"]["name"], "platform");

    let review = fixture.post_json(
        "/v1/admin/portfolio/alethefy/reviews",
        br#"{"confidence":"high","lifecycle":"building","next_action":"cut 0.2.0","blocker":"waiting on the mac","confirm":true}"#,
    );
    assert_eq!(
        review.status,
        200,
        "{}",
        String::from_utf8_lossy(&review.body)
    );

    let detail = body_json(&fixture.get("/v1/admin/portfolio/alethefy", None));
    let profile = &detail["portfolio"]["profile"];
    assert_eq!(profile["lifecycle"], "building");
    assert_eq!(profile["confidence"], "high");
    assert_eq!(profile["next_action"], "cut 0.2.0");
    assert_eq!(profile["blocker"], "waiting on the mac");
    assert_eq!(
        profile["reviewed_at"].as_str().map(str::is_empty),
        Some(false)
    );
    let tags = detail["portfolio"]["tags"].as_array().expect("tags");
    assert_eq!(tags.len(), 1, "{detail}");
    assert_eq!(tags[0]["name"], "platform");
    assert_eq!(
        detail["portfolio"]["reviews"]
            .as_array()
            .expect("reviews")
            .len(),
        1
    );

    // The fleet list shows the same operator metadata.
    let list = body_json(&fixture.get("/v1/admin/portfolio", None));
    let row = project_row(&list, "alethefy");
    assert_eq!(row["lifecycle"], "building");
    assert_eq!(row["confidence"], "high");
    assert_eq!(row["tags"], serde_json::json!(["platform"]));
}

/// Authorization guard 1 — no administrator session.
///
/// Renamed from `..._without_a_bearer_token_...`: the surviving admin
/// gate authenticates the `forge_admin_session` cookie, not a bearer
/// header (`src/api/admin/gateway.rs:456`). The guarantee is unchanged —
/// a request that cannot present the session persists nothing.
#[test]
fn a_portfolio_post_without_an_administrator_session_persists_no_change() {
    let fixture = Fixture::new(&[("alethefy", "rust-web")]);
    let response = fixture.request(
        "POST",
        "/v1/admin/portfolio/alethefy/tags",
        None,
        None,
        Some("application/json"),
        br#"{"name":"platform","confirm":true}"#,
    );
    assert_eq!(response.status, 401);
    assert_eq!(error_code(&response), "api-unauthorized");
    assert_no_portfolio_state(&fixture.db, "alethefy");
}

/// Authorization guard 2 — a hostile `Origin`.
///
/// The exact-origin check runs before the session gate
/// (`src/api/admin/deploy.rs:65`), so a cross-origin write is refused
/// even while riding a valid session cookie.
#[test]
fn a_cross_origin_portfolio_post_persists_no_change() {
    let fixture = Fixture::new(&[("alethefy", "rust-web")]);
    let request = ApiRequest {
        method: "POST".to_string(),
        path: "/v1/admin/portfolio/alethefy/tags".to_string(),
        query: None,
        headers: BTreeMap::from([
            ("origin".to_string(), "http://evil.example.com".to_string()),
            ("content-type".to_string(), "application/json".to_string()),
        ]),
        body: br#"{"name":"platform","confirm":true}"#.to_vec(),
        idempotency_key: None,
        bearer_token: None,
        cookies: BTreeMap::from([("forge_admin_session".to_string(), fixture.token.clone())]),
        remote_addr: Some("127.0.0.1:9999".parse().unwrap()),
        started_at: Utc::now(),
    };
    let response = handle(&fixture.config, &fixture.db, &request, Utc::now());
    assert_eq!(response.status, 403);
    assert_eq!(error_code(&response), "admin-origin-rejected");
    assert_no_portfolio_state(&fixture.db, "alethefy");
}

/// Authorization guard 3 — a form-encoded write.
///
/// The double-submit form token that guarded the deleted HTML form is
/// gone with the form; what replaced it on the surviving JSON route is
/// the content-type gate (`src/api/admin/deploy.rs:154`): the only thing
/// a cross-site HTML `<form>` can send is
/// `application/x-www-form-urlencoded`, and a portfolio write refuses
/// it with `415 admin-content-type-required` before any handler runs —
/// even for a body that carries a complete, confirmed mutation. This is
/// the surviving replacement for the same boundary, so it keeps the
/// test's place in the authorization trio.
#[test]
fn a_form_encoded_portfolio_post_persists_no_change() {
    let fixture = Fixture::new(&[("alethefy", "rust-web")]);
    let response = fixture.request(
        "POST",
        "/v1/admin/portfolio/alethefy/tags",
        None,
        Some(&fixture.token),
        Some("application/x-www-form-urlencoded"),
        b"name=platform&confirm=true",
    );
    assert_eq!(
        response.status,
        415,
        "{}",
        String::from_utf8_lossy(&response.body)
    );
    assert_eq!(error_code(&response), "admin-content-type-required");
    assert_no_portfolio_state(&fixture.db, "alethefy");
}

#[test]
fn an_invalid_portfolio_field_is_a_typed_refusal_and_writes_nothing() {
    let fixture = Fixture::new(&[("alethefy", "rust-web")]);
    for (path, body) in [
        (
            "/v1/admin/portfolio/alethefy/tags",
            &br#"{"name":"Two Words","confirm":true}"#[..],
        ),
        (
            "/v1/admin/portfolio/alethefy/tags",
            &br#"{"name":"","confirm":true}"#[..],
        ),
        (
            "/v1/admin/portfolio/alethefy/reviews",
            &br#"{"confidence":"certain","confirm":true}"#[..],
        ),
        (
            "/v1/admin/portfolio/alethefy/reviews",
            &br#"{"confidence":"high","lifecycle":"shipped","confirm":true}"#[..],
        ),
        // A control character in a free-text field is refused before it
        // reaches the table cell.
        (
            "/v1/admin/portfolio/alethefy/reviews",
            &br#"{"confidence":"high","next_action":"line\nbreak","confirm":true}"#[..],
        ),
        // No editable field at all.
        (
            "/v1/admin/portfolio/alethefy/reviews",
            &br#"{"confirm":true}"#[..],
        ),
        // An out-of-vocabulary relation type.
        (
            "/v1/admin/portfolio/alethefy/relations",
            &br#"{"to":"forge","type":"knows","confirm":true}"#[..],
        ),
    ] {
        let response = fixture.post_json(path, body);
        assert_eq!(response.status, 400, "{} {body:?}", path);
        assert_eq!(
            error_code(&response),
            "portfolio-invalid",
            "{} {body:?}",
            path
        );
        assert_eq!(body_json(&response)["effect"], "none", "{} {body:?}", path);
        assert_no_portfolio_state(&fixture.db, "alethefy");
    }
}

/// Spec: "Unknown project — WHEN an operator writes metadata for an
/// unknown project id THEN Forge returns typed not-found and changes no
/// portfolio state."
#[test]
fn a_portfolio_post_for_an_unknown_project_is_a_typed_404() {
    let fixture = Fixture::new(&[("alethefy", "rust-web")]);
    let response = fixture.post_json(
        "/v1/admin/portfolio/no-such-app/tags",
        br#"{"name":"platform","confirm":true}"#,
    );
    assert_eq!(response.status, 404);
    let body = body_json(&response);
    assert_eq!(body["error"]["code"], "portfolio-unmanaged-project");
    assert_eq!(body["effect"], "none");
    assert_no_portfolio_state(&fixture.db, "alethefy");
    assert_no_portfolio_state(&fixture.db, "no-such-app");
}

/// Imported evidence is source-owned and append-only: a metadata write
/// carrying evidence-shaped keys stores nothing, and the browser's only
/// evidence verb is the confirmed append-only import. A direct edit is
/// refused and the stored snapshot is provably unchanged.
#[test]
fn a_browser_post_never_writes_an_evidence_snapshot() {
    let fixture = Fixture::new(&[("alethefy", "rust-web")]);
    // Even a metadata write that smuggles evidence fields through
    // unknown keys stores no snapshot.
    let response = fixture.post_json(
        "/v1/admin/portfolio/alethefy/tags",
        br#"{"name":"platform","source":"gate","revision":"r1","status":"observed","evidence":{},"confirm":true}"#,
    );
    assert_eq!(
        response.status,
        200,
        "{}",
        String::from_utf8_lossy(&response.body)
    );
    assert!(registry(&fixture.db)
        .portfolio_snapshots_for("alethefy", 10)
        .expect("snapshots")
        .is_empty());

    // One confirmed, append-only import stores a read-only snapshot.
    let imported = fixture.post_json(
        "/v1/admin/portfolio/alethefy/evidence/import",
        br#"{"source_system":"gate","source_revision":"r1","status":"observed","confirm":true}"#,
    );
    assert_eq!(imported.status, 200);
    assert_eq!(
        registry(&fixture.db)
            .portfolio_snapshots_for("alethefy", 10)
            .expect("snapshots")
            .len(),
        1
    );

    // A direct edit through the browser write surface is refused and the
    // stored snapshot is unchanged.
    let edit = fixture.post_json(
        "/v1/admin/portfolio/alethefy/evidence",
        br#"{"source":"gate","revision":"r9","status":"observed","confirm":true}"#,
    );
    assert_eq!(edit.status, 403);
    assert_eq!(error_code(&edit), "portfolio-source-owned");
    assert_eq!(body_json(&edit)["effect"], "none");

    let snapshots = registry(&fixture.db)
        .portfolio_snapshots_for("alethefy", 10)
        .expect("snapshots");
    assert_eq!(
        snapshots.len(),
        1,
        "a refused edit changed the snapshot set"
    );
    assert_eq!(snapshots[0].source_revision, "r1");
    assert_eq!(snapshots[0].source_system, "gate");

    // And every listed row is marked non-editable.
    let evidence = body_json(&fixture.get("/v1/admin/portfolio/alethefy/evidence", None));
    for row in evidence["evidence"].as_array().expect("evidence") {
        assert_eq!(row["editable"], false, "{row}");
    }
}

/// Spec: `web-portfolio-completion` — "Tag and relation removal … each
/// confirm-gated and idempotent: removing an absent entry reports
/// `removed: false` with 200 and changes nothing."
#[test]
fn removing_a_tag_through_the_browser_form_is_idempotent() {
    let fixture = Fixture::new(&[("alethefy", "rust-web")]);
    registry(&fixture.db)
        .portfolio_add_tag("alethefy", "platform", None)
        .expect("tag");

    let body = br#"{"name":"platform","confirm":true}"#;
    let removed = fixture.post_json("/v1/admin/portfolio/alethefy/tags/remove", body);
    assert_eq!(removed.status, 200);
    assert_eq!(body_json(&removed)["result"]["removed"], true);

    let again = fixture.post_json("/v1/admin/portfolio/alethefy/tags/remove", body);
    assert_eq!(again.status, 200);
    assert_eq!(body_json(&again)["result"]["removed"], false);
    assert!(registry(&fixture.db)
        .portfolio_tags_for("alethefy")
        .expect("tags")
        .is_empty());
}

/// A confirmed portfolio metadata write is a registry write and nothing
/// else: it journals no operation row, and it touches no file in the
/// project directory (`portfolio-metadata-and-review` — "no surface
/// edits repository files or provider records").
///
/// The deleted `/ui` portal additionally journaled one `portfolio.ui`
/// row per write. That guarantee came from the `forge-web-human-dashboard`
/// spec, which `7d2df41` deleted with the portal, and no surviving
/// canonical spec requires a journal row for a portfolio metadata write;
/// `write_item` (`src/api/portfolio_writes.rs:37`) journals none. The
/// auditable record is the persisted portfolio row itself plus the
/// `actor`/`recorded_at` the response returns, both asserted below. See
/// the handoff entry for the gap.
#[test]
fn a_browser_portfolio_write_is_recorded_without_a_journal_or_repository_write() {
    let fixture = Fixture::new(&[("alethefy", "rust-web")]);
    let readme = write_project_file(&fixture, "alethefy");
    let before = fs::read(&readme).expect("readme");
    let journal_before = registry(&fixture.db)
        .recent_operations(64)
        .expect("recent")
        .len();

    let response = fixture.post_json(
        "/v1/admin/portfolio/alethefy/tags",
        br#"{"name":"platform","confirm":true}"#,
    );
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    // The write is attributable: who, when, and what it did.
    assert_eq!(body["actor"], "global-admin");
    assert!(body["recorded_at"].as_str().map(str::is_empty) == Some(false));
    assert_eq!(body["effect"], "forge-owned-write");
    assert_eq!(
        registry(&fixture.db)
            .portfolio_tags_for("alethefy")
            .expect("tags")
            .len(),
        1,
        "the operator edit is not recorded in the portfolio store"
    );

    // No operation row is written for a portfolio metadata edit, and in
    // particular no publish-shaped row.
    let recent = registry(&fixture.db).recent_operations(64).expect("recent");
    assert_eq!(
        recent.len(),
        journal_before,
        "a portfolio write journaled an operation row: {recent:?}"
    );
    assert!(
        recent.iter().all(|entry| entry.kind != "publish"),
        "a portfolio write journaled a publish row: {recent:?}"
    );

    // And no repository file moved.
    assert_eq!(fs::read(&readme).expect("readme"), before);
}

// --- compatibility ---------------------------------------------------------

/// Assert no portfolio rows landed. Used after every refused write so a
/// refusal is proven inert, not merely refused.
fn assert_no_portfolio_state(db: &Path, project_id: &str) {
    let registry = Registry::open(db).expect("open registry");
    assert!(
        registry
            .portfolio_tags_for(project_id)
            .expect("tags")
            .is_empty(),
        "a refused write left a tag behind"
    );
    assert!(
        registry
            .portfolio_record(project_id)
            .expect("record")
            .lifecycle
            .is_none(),
        "a refused write left a lifecycle behind"
    );
    assert!(
        registry
            .portfolio_reviews_for(project_id, 10)
            .expect("reviews")
            .is_empty(),
        "a refused write left a review behind"
    );
    assert!(
        registry
            .portfolio_snapshots_for(project_id, 10)
            .expect("snapshots")
            .is_empty(),
        "a refused write left an evidence snapshot behind"
    );
}

/// The old `the_existing_ui_routes_still_answer_without_portfolio_state`
/// existed to prove the legacy `/ui` pages still answered. That premise
/// is void by design: `7d2df41` removed the portal and
/// `portal-spa-convergence` names the SPA as the single interactive
/// surface. Its surviving intent is the half that still holds — the
/// portfolio capability is served without any portfolio state and
/// without a second browser surface — so this pins that instead:
///
/// - both portfolio reads answer `200` for a project that has no
///   portfolio state at all;
/// - the SPA owns `/portfolio`;
/// - `GET /ui` is now an honest `404 route-not-found`, so no second
///   portal can creep back.
#[test]
fn portfolio_reads_answer_without_portfolio_state_and_no_legacy_ui_portal_answers() {
    let fixture = Fixture::new(&[("alethefy", "rust-web")]);

    let list = fixture.get("/v1/admin/portfolio", None);
    assert_eq!(list.status, 200);
    let body = body_json(&list);
    assert_eq!(body["summary"]["projects"], 1);
    assert_eq!(body["summary"]["with_evidence"], 0);
    assert_eq!(body["projects"].as_array().expect("projects").len(), 1);

    let detail = fixture.get("/v1/admin/portfolio/alethefy", None);
    assert_eq!(detail.status, 200);
    let detail = body_json(&detail);
    assert_eq!(detail["portfolio"]["profile"]["project_id"], "alethefy");
    assert_eq!(detail["management"], "managed");
    for empty in ["tags", "goals", "relations", "evidence", "reviews"] {
        assert_eq!(
            detail["portfolio"][empty],
            serde_json::json!([]),
            "`{empty}` is not empty for a project with no portfolio state"
        );
    }

    // The SPA owns the portfolio surface.
    let index = frontend_file("index.html");
    assert!(index.contains("data-route=\"portfolio\" href=\"/portfolio\""));
    assert!(index.contains("id=\"portfolio\""));

    // And no legacy HTML portal answers any more.
    let legacy = fixture.get("/ui", None);
    assert_eq!(legacy.status, 404);
    assert_eq!(error_code(&legacy), "route-not-found");
    for path in ["/ui/projects/alethefy", "/ui/projects/alethefy/portfolio"] {
        let response = fixture.get(path, None);
        assert_eq!(response.status, 404, "{path} still answers");
        assert_eq!(error_code(&response), "route-not-found", "{path}");
    }
}
