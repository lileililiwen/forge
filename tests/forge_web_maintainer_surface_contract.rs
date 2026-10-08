//! Contract tests for the browser maintainer surface
//! (`forge-web-maintainer-surface`).
//!
//! These drive the real handlers so the two additions are proven end to
//! end:
//!
//! - The fleet filter row (language, lifecycle, profile, compose, CI,
//!   tag) is wired to `GET /v1/projects/catalog`: for the same
//!   predicate the handler returns the same rows as the equivalent
//!   `forge project list` CLI invocation, and the predicates compose
//!   as AND. The free-text search composes client-side on top of the
//!   returned id set (`renderProjects` in `frontend/app.js`).
//! - The per-project Maintain card data route
//!   (`GET /v1/admin/projects/{id}/maintain`) renders the observed
//!   GitHub description/topics/homepage/language with freshness, the
//!   derived proposals with per-field approve/reject, and the one
//!   apply action over the approved set — all through the
//!   preview → confirm → apply discipline `buildActionControl`
//!   renders from the command catalog.
//! - An unreachable remote renders `unavailable` with its reason,
//!   never empty fields that read as "no description set".

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use chrono::Utc;
use forge::api::{handle, ApiConfig, ApiRequest};
use forge::registry::Registry;
use serde_json::Value;
use tempfile::TempDir;

/// Environment touches are process-global, so every test in this binary
/// serializes on one lock.
static SERIAL: Mutex<()> = Mutex::new(());
fn lock() -> std::sync::MutexGuard<'static, ()> {
    match SERIAL.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

const EMAIL: &str = "maintainer-operator@example.test";
const PASSWORD: &str = "a-long-test-password";
const MAINTAIN_CONTRACT: &str = "forge-project-maintain/0.1.0";

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn clear_env() {
    for key in [
        "FORGE_REGISTRY",
        "FORGE_WORKSPACE_REGISTRY",
        "FORGE_INVENTORY_SOURCE",
        "FORGE_GITHUB_TOKEN",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
    ] {
        std::env::remove_var(key);
    }
}

fn request(method: &str, path: &str, query: Option<&str>, origin: &str) -> ApiRequest {
    ApiRequest {
        method: method.to_string(),
        path: path.to_string(),
        query: query.map(str::to_string),
        headers: BTreeMap::from([("origin".to_string(), origin.to_string())]),
        body: Vec::new(),
        idempotency_key: None,
        bearer_token: None,
        cookies: BTreeMap::new(),
        remote_addr: None,
        started_at: Utc::now(),
    }
}

fn with_session(request: &mut ApiRequest, token: &str) {
    request
        .cookies
        .insert("forge_admin_session".to_string(), token.to_string());
}

fn post(request: &mut ApiRequest, body: &Value) {
    request
        .headers
        .insert("content-type".to_string(), "application/json".to_string());
    request.body = body.to_string().into_bytes();
}

fn setup_admin(db: &Path) {
    forge::identity::global::setup(db, EMAIL, PASSWORD).unwrap();
}

fn login_token(config: &ApiConfig, db: &Path) -> String {
    let mut login = request("POST", "/v1/admin/session", None, &config.frontend_origin);
    post(
        &mut login,
        &serde_json::json!({ "email": EMAIL, "password": PASSWORD }),
    );
    let response = handle(config, db, &login, Utc::now());
    assert_eq!(response.status, 200, "login must succeed");
    response
        .headers
        .get("set-cookie")
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .split_once('=')
        .unwrap()
        .1
        .to_string()
}

fn body_json(response: &forge::api::ApiResponse) -> Value {
    serde_json::from_slice(&response.body).expect("responses are JSON")
}

/// One registered project: `compose`/`docker` marker files, an optional
/// GitHub Actions workflow, and an optional installed feature (which the
/// catalog projects as a tag).
fn write_project(
    parent: &Path,
    id: &str,
    profile: &str,
    maturity: &str,
    language: &str,
    compose: bool,
    docker: bool,
    ci: bool,
    features: bool,
) -> PathBuf {
    let dir = parent.join(id);
    std::fs::create_dir_all(&dir).unwrap();
    let features_yaml = if features {
        "features:\n  auth: 0.1.0\n"
    } else {
        ""
    };
    std::fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: {profile}\n  \
             maturity: {maturity}\n  target_maturity: {maturity}\nruntime:\n  language: {language}\n{features_yaml}"
        ),
    )
    .unwrap();
    std::fs::write(dir.join("README.md"), format!("# {id}\n")).unwrap();
    if compose {
        std::fs::write(dir.join("compose.yaml"), "services: {}\n").unwrap();
    }
    if docker {
        std::fs::write(dir.join("Dockerfile"), "FROM scratch\n").unwrap();
    }
    if ci {
        let workflows = dir.join(".github/workflows");
        std::fs::create_dir_all(&workflows).unwrap();
        std::fs::write(workflows.join("ci.yml"), "on: push\n").unwrap();
    }
    dir
}

fn register(db: &Path, dir: &Path) {
    let mut registry = Registry::open(db).unwrap();
    registry.register(dir, None).unwrap();
}

/// Three registered projects the predicates separate: `alpha` ships
/// compose + CI + a feature tag, `beta` is a Dockerfile-only Python
/// service, `gamma` ships neither compose nor CI.
fn seed_fleet(tmp: &Path) -> (PathBuf, Vec<PathBuf>) {
    let db = tmp.join("registry.db");
    let projects = tmp.join("projects");
    let alpha = write_project(
        &projects, "alpha", "rust-web", "L1", "rust", true, true, true, true,
    );
    let beta = write_project(
        &projects,
        "beta",
        "python-service",
        "L3",
        "python",
        false,
        true,
        false,
        false,
    );
    let gamma = write_project(
        &projects, "gamma", "rust-web", "L2", "rust", false, false, false, false,
    );
    for dir in [&alpha, &beta, &gamma] {
        register(&db, dir);
    }
    (db, vec![alpha, beta, gamma])
}

fn cli_list_ids(db: &Path, args: &[&str]) -> BTreeSet<String> {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.env_remove("FORGE_WORKSPACE_REGISTRY");
    cmd.env_remove("FORGE_INVENTORY_SOURCE");
    cmd.env_remove("HTTP_PROXY");
    cmd.env_remove("HTTPS_PROXY");
    cmd.env_remove("ALL_PROXY");
    cmd.arg("--registry").arg(db).arg("project").arg("list");
    for arg in args {
        cmd.arg(arg);
    }
    cmd.arg("--format").arg("json");
    let out = cmd.output().expect("run forge");
    assert!(
        out.status.success(),
        "forge project list {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let page: Value = serde_json::from_slice(&out.stdout).expect("cli json");
    page["catalog"]["records"]
        .as_array()
        .expect("records array")
        .iter()
        .map(|record| record["project_id"].as_str().unwrap().to_string())
        .collect()
}

fn catalog_ids(config: &ApiConfig, db: &Path, token: &str, query: &str) -> BTreeSet<String> {
    let mut fetch = request(
        "GET",
        "/v1/projects/catalog",
        Some(query),
        &config.frontend_origin,
    );
    // The browser presents its administrator session cookie; it cannot
    // present a Bearer header (HttpOnly). No Bearer token is set here
    // on purpose: this is the exact shape of the filter-row fetch.
    with_session(&mut fetch, token);
    let response = handle(config, db, &fetch, Utc::now());
    assert_eq!(
        response.status,
        200,
        "catalog query `{query}` refused: {}",
        String::from_utf8_lossy(&response.body)
    );
    body_json(&response)["catalog"]["records"]
        .as_array()
        .expect("records array")
        .iter()
        .map(|record| record["project_id"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn fleet_filter_row_matches_forge_project_list_for_the_same_predicate() {
    let _guard = lock();
    clear_env();
    let tmp = TempDir::new().unwrap();
    let (db, _dirs) = seed_fleet(tmp.path());
    setup_admin(&db);
    let config = ApiConfig::default();
    let token = login_token(&config, &db);

    // (CLI flags, catalog query string): the filter row builds the query
    // string; `forge project list` builds the same pairs from flags.
    let cases: &[(&[&str], &str)] = &[
        (&["--language", "rust"], "language=rust&limit=1000"),
        (&["--lifecycle", "L1"], "lifecycle=L1&limit=1000"),
        (&["--profile", "rust-web"], "profile=rust-web&limit=1000"),
        (
            &["--compose", "docker+compose"],
            "compose=docker%2Bcompose&limit=1000",
        ),
        (&["--ci", "github-actions"], "ci=github-actions&limit=1000"),
        (&["--tag", "auth"], "tag=auth&limit=1000"),
        // Predicates compose as AND on both surfaces.
        (
            &["--language", "rust", "--compose", "docker+compose"],
            "language=rust&compose=docker%2Bcompose&limit=1000",
        ),
        (
            &["--profile", "rust-web", "--tag", "auth"],
            "profile=rust-web&tag=auth&limit=1000",
        ),
    ];
    for (flags, query) in cases {
        let from_cli = cli_list_ids(&db, flags);
        let from_row = catalog_ids(&config, &db, &token, query);
        assert_eq!(
            from_row, from_cli,
            "filter row diverges from `forge project list {flags:?}`"
        );
    }
    // Assert the discriminating cases explicitly so a both-empty
    // agreement can never pass for the wrong reason.
    assert_eq!(
        catalog_ids(&config, &db, &token, "language=rust&limit=1000"),
        BTreeSet::from(["alpha".to_string(), "gamma".to_string()])
    );
    assert_eq!(
        catalog_ids(&config, &db, &token, "compose=docker%2Bcompose&limit=1000"),
        BTreeSet::from(["alpha".to_string()])
    );
    assert_eq!(
        catalog_ids(
            &config,
            &db,
            &token,
            "language=rust&compose=docker%2Bcompose&limit=1000"
        ),
        BTreeSet::from(["alpha".to_string()])
    );
}

#[test]
fn browser_session_cookie_reads_catalog_without_bearer() {
    let _guard = lock();
    clear_env();
    let tmp = TempDir::new().unwrap();
    let (db, _dirs) = seed_fleet(tmp.path());
    setup_admin(&db);
    let config = ApiConfig::default();

    // No session at all still 401s.
    let anonymous = request(
        "GET",
        "/v1/projects/catalog",
        Some("language=rust&limit=1000"),
        &config.frontend_origin,
    );
    let response = handle(&config, &db, &anonymous, Utc::now());
    assert_eq!(response.status, 401);

    // A hostile origin presenting a valid cookie is still refused.
    let token = login_token(&config, &db);
    let mut attacker = request(
        "GET",
        "/v1/projects/catalog",
        Some("language=rust&limit=1000"),
        "https://evil.example",
    );
    with_session(&mut attacker, &token);
    let response = handle(&config, &db, &attacker, Utc::now());
    assert_eq!(response.status, 403);

    // The browser's cookie session reads without any Bearer header.
    let mut fetch = request(
        "GET",
        "/v1/projects/catalog",
        Some("language=rust&limit=1000"),
        &config.frontend_origin,
    );
    with_session(&mut fetch, &token);
    let response = handle(&config, &db, &fetch, Utc::now());
    assert_eq!(response.status, 200);
    let records = body_json(&response)["catalog"]["records"]
        .as_array()
        .expect("records array")
        .len();
    assert_eq!(records, 2, "language=rust matches alpha and gamma");
}

/// Preview-then-confirm approve of one proposal through the Maintain
/// card's action route, asserting the digest-bound discipline holds.
fn approve_proposal(config: &ApiConfig, db: &Path, token: &str, id: &str, proposal: &str) {
    let mut preview = request(
        "POST",
        &format!("/v1/admin/projects/{id}/classify/approve"),
        None,
        &config.frontend_origin,
    );
    with_session(&mut preview, token);
    post(&mut preview, &serde_json::json!({ "proposal": proposal }));
    let response = handle(config, db, &preview, Utc::now());
    assert_eq!(response.status, 200, "approve preview refused");
    let digest = body_json(&response)["plan_digest"]
        .as_str()
        .expect("preview digest")
        .to_string();
    let mut approve = request(
        "POST",
        &format!("/v1/admin/projects/{id}/classify/approve"),
        None,
        &config.frontend_origin,
    );
    with_session(&mut approve, token);
    post(
        &mut approve,
        &serde_json::json!({
            "proposal": proposal,
            "confirm": true,
            "plan_digest": digest,
        }),
    );
    let response = handle(config, db, &approve, Utc::now());
    assert_eq!(response.status, 200, "approve refused");
    assert_eq!(body_json(&response)["state"], "approved");
}

fn maintain(config: &ApiConfig, db: &Path, token: &str, id: &str) -> Value {
    let mut fetch = request(
        "GET",
        &format!("/v1/admin/projects/{id}/maintain"),
        None,
        &config.frontend_origin,
    );
    with_session(&mut fetch, token);
    let response = handle(config, db, &fetch, Utc::now());
    assert_eq!(
        response.status,
        200,
        "maintain refused: {}",
        String::from_utf8_lossy(&response.body)
    );
    body_json(&response)
}

#[test]
fn maintain_card_renders_observation_freshness_proposals_and_apply() {
    let _guard = lock();
    clear_env();
    let tmp = TempDir::new().unwrap();
    let (db, dirs) = seed_fleet(tmp.path());
    setup_admin(&db);
    let config = ApiConfig::default();
    let token = login_token(&config, &db);

    // Derive proposals from the alpha manifest so the card has something
    // reviewable; deriving writes proposals only, never project fields.
    let derived = forge::semantic::derive(&dirs[0]).expect("derive");
    assert!(!derived.is_empty(), "derive must record proposals");

    let card = maintain(&config, &db, &token, "alpha");
    assert_eq!(card["contract"], MAINTAIN_CONTRACT);
    assert_eq!(card["project_id"], "alpha");

    // No GitHub remote is declared and no adapter/token is configured in
    // this environment, so the observation is honestly unavailable.
    let github = &card["github"];
    assert_eq!(github["state"], "unavailable");
    assert!(
        github["reason"]
            .as_str()
            .is_some_and(|reason| !reason.is_empty()),
        "an unreachable remote names its reason: {github}"
    );
    for field in ["description", "topics", "homepage", "language"] {
        assert!(
            github.get(field).is_none(),
            "unreachable remote must not render an empty `{field}`: {github}"
        );
    }

    // The derived proposals carry everything the per-field approve/reject
    // needs: the proposal id, its kind, state and confidence.
    let proposals = card["proposals"].as_array().expect("proposals array");
    assert!(!proposals.is_empty(), "derived proposals must render");
    for proposal in proposals {
        for field in ["id", "kind", "state", "confidence"] {
            assert!(
                proposal[field]
                    .as_str()
                    .is_some_and(|value| !value.is_empty()),
                "proposal misses `{field}`: {proposal}"
            );
        }
    }
    assert!(
        proposals
            .iter()
            .any(|proposal| proposal["state"] == "suggested"),
        "a fresh derive leaves suggested proposals: {proposals:?}"
    );
    assert!(card["plugins"].is_array(), "plugin registry must render");

    // The one apply action previews the approved set (currently empty)
    // without sending anything anywhere.
    let mut preview = request(
        "POST",
        "/v1/admin/projects/alpha/classify/apply",
        None,
        &config.frontend_origin,
    );
    with_session(&mut preview, &token);
    post(&mut preview, &serde_json::json!({}));
    let response = handle(&config, &db, &preview, Utc::now());
    assert_eq!(response.status, 200, "apply preview refused");
    let preview_body = body_json(&response);
    assert_eq!(preview_body["effect"], "none");
    assert!(preview_body["plan_digest"].as_str().is_some());
    assert_eq!(
        preview_body["preview"]["approved_proposals"]
            .as_array()
            .expect("approved array")
            .len(),
        0
    );

    // A Suggested proposal is refused by name until it is approved: first
    // the preview binds the digest, then the confirmation approves it.
    // Every derived proposal is approved so the later apply reaches the
    // field boundary instead of stopping at the first Suggested name.
    for entry in forge::semantic::list(&dirs[0]).expect("list") {
        approve_proposal(&config, &db, &token, "alpha", &entry.dir_name);
    }

    // The now-approved set previews with one entry; confirming refuses
    // at the field boundary — a `profile` approval is outside the four
    // permitted metadata fields — rather than silently dropping it. The
    // refusal is typed (`semantic-invalid`); the transport status follows
    // the shared `from_error` mapping.
    let mut apply_preview = request(
        "POST",
        "/v1/admin/projects/alpha/classify/apply",
        None,
        &config.frontend_origin,
    );
    with_session(&mut apply_preview, &token);
    post(&mut apply_preview, &serde_json::json!({}));
    let response = handle(&config, &db, &apply_preview, Utc::now());
    assert_eq!(response.status, 200);
    let apply_body = body_json(&response);
    let apply_digest = apply_body["plan_digest"]
        .as_str()
        .expect("apply digest")
        .to_string();
    assert!(
        apply_body["preview"]["approved_proposals"]
            .as_array()
            .expect("approved array")
            .iter()
            .any(|id| id.as_str().is_some_and(|id| id.starts_with("profile-"))),
        "the approved proposals preview: {apply_body}"
    );
    let mut apply = request(
        "POST",
        "/v1/admin/projects/alpha/classify/apply",
        None,
        &config.frontend_origin,
    );
    with_session(&mut apply, &token);
    post(
        &mut apply,
        &serde_json::json!({ "confirm": true, "plan_digest": apply_digest }),
    );
    let response = handle(&config, &db, &apply, Utc::now());
    let refusal = body_json(&response);
    assert_eq!(refusal["error"]["code"], "semantic-invalid");
    assert!(
        refusal["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("outside the permitted set")),
        "apply refusal must name the field boundary: {refusal}"
    );

    // On `gamma` a description proposal — inside the permitted fields —
    // is suggested and approved, so confirming the apply reaches the
    // plugin boundary: no plugin advertises `kind: metadata` here, and
    // that refusal names the capability instead of discarding the
    // approved value.
    let outcome = forge::semantic::suggest(&forge::semantic::SuggestRequest {
        project_path: dirs[2].clone(),
        kind: forge::semantic::ProposalKind::Description,
        current_value: None,
        suggested_value: "A demo service".to_string(),
        confidence: forge::semantic::Confidence::Medium,
        provider: forge::semantic::Provider::Local,
        evidence: vec![
            forge::semantic::ProposalEvidence::new("forge.yaml", "rev", "test").unwrap(),
        ],
        note: None,
        now: Utc::now(),
    })
    .expect("suggest description");
    assert!(outcome.proposal.is_some(), "description must be recorded");
    let entries = forge::semantic::list(&dirs[2]).expect("list gamma");
    let description = entries
        .iter()
        .find(|entry| entry.dir_name.starts_with("description-"))
        .expect("a description proposal");
    let gamma_preview = {
        let mut preview = request(
            "POST",
            "/v1/admin/projects/gamma/classify/approve",
            None,
            &config.frontend_origin,
        );
        with_session(&mut preview, &token);
        post(
            &mut preview,
            &serde_json::json!({ "proposal": description.dir_name }),
        );
        let response = handle(&config, &db, &preview, Utc::now());
        assert_eq!(response.status, 200);
        body_json(&response)["plan_digest"]
            .as_str()
            .expect("gamma digest")
            .to_string()
    };
    let mut gamma_approve = request(
        "POST",
        "/v1/admin/projects/gamma/classify/approve",
        None,
        &config.frontend_origin,
    );
    with_session(&mut gamma_approve, &token);
    post(
        &mut gamma_approve,
        &serde_json::json!({
            "proposal": description.dir_name,
            "confirm": true,
            "plan_digest": gamma_preview,
        }),
    );
    let response = handle(&config, &db, &gamma_approve, Utc::now());
    assert_eq!(response.status, 200, "gamma approve refused");
    let gamma_apply_preview = {
        let mut preview = request(
            "POST",
            "/v1/admin/projects/gamma/classify/apply",
            None,
            &config.frontend_origin,
        );
        with_session(&mut preview, &token);
        post(&mut preview, &serde_json::json!({}));
        let response = handle(&config, &db, &preview, Utc::now());
        assert_eq!(response.status, 200);
        body_json(&response)["plan_digest"]
            .as_str()
            .expect("gamma apply digest")
            .to_string()
    };
    let mut gamma_apply = request(
        "POST",
        "/v1/admin/projects/gamma/classify/apply",
        None,
        &config.frontend_origin,
    );
    with_session(&mut gamma_apply, &token);
    post(
        &mut gamma_apply,
        &serde_json::json!({ "confirm": true, "plan_digest": gamma_apply_preview }),
    );
    let response = handle(&config, &db, &gamma_apply, Utc::now());
    let refusal = body_json(&response);
    assert_eq!(refusal["error"]["code"], "semantic-invalid");
    assert!(
        refusal["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("kind: metadata")),
        "apply refusal must name the missing capability: {refusal}"
    );
}

#[test]
fn unreachable_remote_renders_unavailable_with_reason_never_empty_fields() {
    let _guard = lock();
    clear_env();
    let tmp = TempDir::new().unwrap();
    let (db, _dirs) = seed_fleet(tmp.path());
    setup_admin(&db);
    let config = ApiConfig::default();
    let token = login_token(&config, &db);

    // `beta` has no GitHub remote at all: the card states that fact.
    let card = maintain(&config, &db, &token, "beta");
    let github = &card["github"];
    assert_eq!(github["state"], "unavailable");
    assert!(
        github["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("no GitHub remote")),
        "the reason names the missing remote: {github}"
    );
    assert_eq!(github["freshness"], "unknown");
    assert!(github["observed_at"].is_null());
    for field in ["description", "topics", "homepage", "language", "languages"] {
        assert!(
            github.get(field).is_none(),
            "unreachable remote must not render an empty `{field}`: {github}"
        );
    }

    // A project whose remote points at GitHub but has no reachable
    // adapter/token is unavailable for a different, equally explicit
    // reason — still never blank fields.
    let remote_dir = tmp.path().join("projects").join("delta");
    std::fs::create_dir_all(&remote_dir).unwrap();
    std::fs::write(
        remote_dir.join("forge.yaml"),
        "schema: 1\nproject:\n  id: delta\n  name: delta\n  profile: rust-web\n  maturity: L2\n  target_maturity: L2\n",
    )
    .unwrap();
    let git = |args: &[&str]| {
        let status = Command::new("git")
            .arg("-C")
            .arg(&remote_dir)
            .args(args)
            .status()
            .expect("git must run");
        assert!(status.success(), "git {args:?} failed");
    };
    git(&["init", "-q"]);
    git(&[
        "remote",
        "add",
        "origin",
        "https://github.com/example/demo.git",
    ]);
    register(&db, &remote_dir);
    let card = maintain(&config, &db, &token, "delta");
    let github = &card["github"];
    assert_eq!(github["state"], "unavailable");
    assert!(
        github["reason"].as_str().is_some_and(|reason| {
            reason.contains("FORGE_GITHUB_TOKEN") || reason.contains("adapter")
        }),
        "the reason names the missing credential or adapter: {github}"
    );
    for field in ["description", "topics", "homepage", "language", "languages"] {
        assert!(
            github.get(field).is_none(),
            "unreachable remote must not render an empty `{field}`: {github}"
        );
    }
}

#[test]
fn maintainer_surface_frontend_wiring() {
    // The browser half has no Rust surface, so this pins its wiring
    // instead: the catalog endpoint the filter row reads, the maintain
    // route and element ids the card renders into, and the catalog
    // actions the card reuses. A renamed id or a bespoke endpoint
    // breaks this before it breaks silently in a browser.
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let app = std::fs::read_to_string(root.join("frontend/app.js")).expect("app.js");
    let page = std::fs::read_to_string(root.join("frontend/index.html")).expect("index.html");
    for token in [
        "/v1/projects/catalog",
        "/maintain",
        "fleetFilterIds",
        "wb-maintain-body",
        "wb-maintain-actions",
        "classify.approve",
        "classify.reject",
        "classify.apply",
        "buildActionControl",
    ] {
        assert!(
            app.contains(token),
            "frontend/app.js no longer wires `{token}`"
        );
    }
    for id in [
        "filter-language",
        "filter-lifecycle",
        "filter-profile",
        "filter-compose",
        "filter-ci",
        "filter-tag",
        "filter-clear",
        "fleet-filter-error",
        "wb-maintain-body",
        "wb-maintain-actions",
        "wb-maintain-refresh",
    ] {
        assert!(
            page.contains(&format!("id=\"{id}\"")),
            "frontend/index.html no longer carries `#{id}`"
        );
    }
}
