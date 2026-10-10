//! Read-only agent + identity session visibility contract
//! (`web-agent-identity-readonly`).
//!
//! Pins the five typed, session-gated GET routes —
//! `GET /v1/admin/projects/{id}/agents`,
//! `GET /v1/admin/projects/{id}/agents/{session_id}`,
//! `GET /v1/admin/projects/{id}/identity/config`,
//! `GET /v1/admin/projects/{id}/identity/sessions` and
//! `GET /v1/admin/projects/{id}/identity/sessions/{session_id}` —
//! to the same read boundary the shipped history routes honour: an
//! anonymous request is refused with `401` before any Core call, a
//! hostile project id is a typed `400` without echo, an unmanaged id
//! a `404`, a bad session id a typed `400` without echo, an unknown
//! session id a typed `404`, and a sibling-owned identity session a
//! typed `403`. Lists read back seeded persisted state (written
//! through the same file shapes Core persists); the agent status
//! projects out the absolute `project_path`, reports `live: null`
//! (the adapter subprocess is never spawned) and surfaces
//! spawn-free adapter availability; the config read never contacts a
//! provider. No route writes state, journals, builds a challenge,
//! mints or revokes anything, and no response ever serializes the
//! absolute project path or secret material. Static-token pins cover
//! the five catalog rows (`web`, count stays 234), the
//! `session-terminate` → `cli_only` conversion, the untouched
//! lifecycle/secret/transport verbs, and the Workbench card (no new
//! dependency, no `innerHTML` in the added controls).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use forge::api::{command_catalog, handle, ApiConfig, ApiRequest};
use forge::registry::Registry;
use serde_json::json;

fn app_js() -> String {
    std::fs::read_to_string("frontend/app.js").expect("frontend/app.js ships")
}

fn index_html() -> String {
    std::fs::read_to_string("frontend/index.html").expect("frontend/index.html ships")
}

fn request(method: &str, path: &str, origin: &str) -> ApiRequest {
    ApiRequest {
        method: method.to_string(),
        path: path.to_string(),
        query: None,
        headers: BTreeMap::from([("origin".to_string(), origin.to_string())]),
        body: Vec::new(),
        idempotency_key: None,
        bearer_token: None,
        cookies: BTreeMap::new(),
        remote_addr: None,
        started_at: Utc::now(),
    }
}

fn with_cookie(mut req: ApiRequest, token: &str) -> ApiRequest {
    req.cookies
        .insert("forge_admin_session".to_string(), token.to_string());
    req
}

fn json_body(mut req: ApiRequest, body: &[u8]) -> ApiRequest {
    req.headers
        .insert("content-type".to_string(), "application/json".to_string());
    req.body = body.to_vec();
    req
}

fn login_token(config: &ApiConfig, db: &Path) -> String {
    forge::identity::global::setup(db, "operator@example.test", "a-long-test-password").unwrap();
    let mut login = request("POST", "/v1/admin/session", &config.frontend_origin);
    login = json_body(
        login,
        br#"{"email":"operator@example.test","password":"a-long-test-password"}"#,
    );
    let response = handle(config, db, &login, Utc::now());
    assert_eq!(response.status, 200);
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

fn body_json(response: &forge::api::ApiResponse) -> serde_json::Value {
    serde_json::from_slice(&response.body).expect("admin routes are JSON-only")
}

fn agent_identity_paths(id: &str) -> Vec<String> {
    vec![
        format!("/v1/admin/projects/{id}/agents"),
        format!("/v1/admin/projects/{id}/agents/main-session"),
        format!("/v1/admin/projects/{id}/identity/config"),
        format!("/v1/admin/projects/{id}/identity/sessions"),
        format!("/v1/admin/projects/{id}/identity/sessions/abcdef0123456789"),
    ]
}

const IDENTITY_BLOCK: &str = "identity:\n  provider: okta\n  issuer: https://example.okta.com\n  client_id: forge-admin\n  audience: forge-admin\n  redirect_uri: https://admin.example.com/oidc/callback\n  scopes:\n    - openid\n    - profile\n  admin_claim: groups\n  admin_values:\n    - forge-admins\n  state_ttl_seconds: 120\n  session_ttl_seconds: 3600\n  client_secret_ref: env://OIDC_CLIENT_SECRET\n";

fn write_project(dir: &Path, id: &str, identity: Option<&str>) {
    fs::create_dir_all(dir).unwrap();
    let mut manifest = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n"
    );
    if let Some(block) = identity {
        manifest.push_str(block);
    }
    fs::write(dir.join("forge.yaml"), manifest).unwrap();
}

fn fixture() -> (tempfile::TempDir, PathBuf, ApiConfig) {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let project = tmp.path().join("p1");
    write_project(&project, "p1", Some(IDENTITY_BLOCK));
    {
        let mut registry = Registry::open(&db).unwrap();
        registry.register(&project, None).unwrap();
    }
    let config = ApiConfig::default();
    (tmp, db, config)
}

fn fixture_time() -> chrono::DateTime<Utc> {
    chrono::DateTime::parse_from_rfc3339("2026-01-15T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

/// Seed one recorded agent session through the same file shape Core
/// persists, so the agent routes exercise the real loader without
/// running any adapter.
fn seed_agent_session(project: &Path) {
    let now = fixture_time();
    let session = forge::agent::AgentSession {
        contract: forge::agent::AGENT_CONTRACT_VERSION.to_string(),
        session_id: "main-session".to_string(),
        project_id: "p1".to_string(),
        project_path: project.display().to_string(),
        provider: forge::agent::AgentProvider::Opencode,
        spec_id: "spec-1".to_string(),
        state: forge::agent::SessionState::Idle,
        started_at: now,
        last_transition_at: now,
        transitions: Vec::new(),
        backing: None,
    };
    forge::agent::write_session(project, &session).unwrap();
}

/// Seed one persisted identity session through the same file shape
/// Core persists, so the identity routes exercise the real loader
/// without minting anything.
fn seed_identity_session(project: &Path, project_id: &str, session_id: &str) {
    let now = fixture_time();
    let session = forge::identity::AdminSession {
        session_id: session_id.to_string(),
        project_id: project_id.to_string(),
        provider: "okta".to_string(),
        subject: "user-1".to_string(),
        issued_at: now,
        expires_at: now + chrono::Duration::seconds(3600),
        permissions: vec!["admin:access".to_string()],
        state: forge::identity::SessionState::Active,
        note: None,
    };
    forge::identity::save_session(project, project_id, &session).unwrap();
}

fn get(config: &ApiConfig, db: &Path, token: &str, path: &str) -> forge::api::ApiResponse {
    let req = with_cookie(request("GET", path, &config.frontend_origin), token);
    handle(config, db, &req, Utc::now())
}

fn journal_len(db: &Path) -> usize {
    let registry = Registry::open(db).unwrap();
    registry.recent_operations(500).unwrap().len()
}

#[test]
fn anonymous_requests_are_refused_before_any_core_call() {
    let (_tmp, db, config) = fixture();
    forge::identity::global::setup(&db, "operator@example.test", "a-long-test-password").unwrap();
    for path in agent_identity_paths("p1") {
        let response = handle(
            &config,
            &db,
            &request("GET", &path, &config.frontend_origin),
            Utc::now(),
        );
        assert_eq!(response.status, 401, "anonymous {path} refused");
        assert_eq!(
            body_json(&response)["error"]["code"],
            "api-unauthorized",
            "anonymous {path} typed"
        );
        let raw = String::from_utf8_lossy(&response.body);
        assert!(!raw.contains("session_id"), "no session data leaks: {path}");
    }
}

#[test]
fn hostile_id_is_refused_and_unmanaged_id_is_not_found() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    for path in agent_identity_paths("..%2F..%2Fetc") {
        let response = get(&config, &db, &token, &path);
        assert_eq!(response.status, 400, "hostile {path} refused");
        assert_eq!(
            body_json(&response)["error"]["code"],
            "admin-invalid-project-id",
            "hostile {path} typed"
        );
        let raw = String::from_utf8_lossy(&response.body);
        assert!(!raw.contains(".."), "hostile id echoed: {raw}");
    }
    for path in agent_identity_paths("ghost-proj") {
        let response = get(&config, &db, &token, &path);
        assert_eq!(response.status, 404, "unmanaged {path} not found");
        assert_eq!(
            body_json(&response)["error"]["code"],
            "admin-project-unmanaged",
            "unmanaged {path} typed"
        );
    }
}

#[test]
fn agent_list_is_empty_then_reads_back_seeded_sessions() {
    let (tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let project = tmp.path().join("p1");
    let response = get(&config, &db, &token, "/v1/admin/projects/p1/agents");
    assert_eq!(response.status, 200);
    assert_eq!(body_json(&response)["agent_sessions"], json!([]));
    assert_eq!(
        body_json(&response)["contract"],
        forge::api::API_CONTRACT_VERSION
    );

    seed_agent_session(&project);
    let response = get(&config, &db, &token, "/v1/admin/projects/p1/agents");
    assert_eq!(response.status, 200);
    let entries = body_json(&response)["agent_sessions"].clone();
    assert_eq!(entries.as_array().unwrap().len(), 1);
    assert_eq!(entries[0]["session_id"], json!("main-session"));
    assert_eq!(entries[0]["spec_id"], json!("spec-1"));
}

#[test]
fn agent_status_resolves_missing_and_bad_ids_honestly() {
    let (tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let project = tmp.path().join("p1");
    seed_agent_session(&project);

    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/p1/agents/main-session",
    );
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    assert_eq!(body["session"]["session_id"], json!("main-session"));
    assert_eq!(body["session"]["spec_id"], json!("spec-1"));
    assert_eq!(body["live"], serde_json::Value::Null);
    assert!(
        body["live_note"]
            .as_str()
            .unwrap_or("")
            .contains("recorded"),
        "honesty note must say the browser renders recorded state"
    );
    assert!(body["adapter"]["available"].is_boolean());
    assert_eq!(body["contract"], forge::api::API_CONTRACT_VERSION);
    let raw = String::from_utf8_lossy(&response.body);
    assert!(
        !raw.contains(&project.display().to_string()),
        "project path leaked"
    );
    assert!(
        !body["session"]
            .as_object()
            .unwrap()
            .contains_key("project_path"),
        "project_path must be projected out"
    );

    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/p1/agents/no-such-session",
    );
    assert_eq!(response.status, 404);
    assert_eq!(
        body_json(&response)["error"]["code"],
        "admin-agent-session-not-found"
    );

    let response = get(&config, &db, &token, "/v1/admin/projects/p1/agents/..");
    assert_eq!(response.status, 400);
    assert_eq!(
        body_json(&response)["error"]["code"],
        "admin-invalid-agent-session-id"
    );
    assert!(!String::from_utf8_lossy(&response.body).contains(".."));
}

#[test]
fn identity_session_list_is_empty_then_reads_back_seeded_sessions() {
    let (tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let project = tmp.path().join("p1");
    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/p1/identity/sessions",
    );
    assert_eq!(response.status, 200);
    assert_eq!(body_json(&response)["sessions"], json!([]));

    seed_identity_session(&project, "p1", "abcdef0123456789");
    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/p1/identity/sessions",
    );
    assert_eq!(response.status, 200);
    let entries = body_json(&response)["sessions"].clone();
    assert_eq!(entries.as_array().unwrap().len(), 1);
    assert_eq!(entries[0]["session_id"], json!("abcdef0123456789"));
    assert_eq!(entries[0]["subject"], json!("user-1"));
    assert_eq!(
        body_json(&response)["contract"],
        forge::api::API_CONTRACT_VERSION
    );
}

#[test]
fn identity_inspect_resolves_missing_bad_and_cross_project_honestly() {
    let (tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let p1 = tmp.path().join("p1");
    seed_identity_session(&p1, "p1", "abcdef0123456789");

    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/p1/identity/sessions/abcdef0123456789",
    );
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    assert_eq!(body["session"]["subject"], json!("user-1"));
    assert_eq!(body["session"]["project_id"], json!("p1"));

    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/p1/identity/sessions/ffffffffffffffff",
    );
    assert_eq!(response.status, 404);
    assert_eq!(
        body_json(&response)["error"]["code"],
        "identity-session-not-found"
    );

    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/p1/identity/sessions/not-hex!!",
    );
    assert_eq!(response.status, 400);
    assert_eq!(
        body_json(&response)["error"]["code"],
        "admin-invalid-identity-session-id"
    );

    // A sibling-owned session is a cross-project refusal, not a 404.
    let p2 = tmp.path().join("p2");
    write_project(&p2, "p2", Some(IDENTITY_BLOCK));
    {
        let mut registry = Registry::open(&db).unwrap();
        registry.register(&p2, None).unwrap();
    }
    seed_identity_session(&p2, "p2", "0011223344556677");
    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/p1/identity/sessions/0011223344556677",
    );
    assert_eq!(response.status, 403);
    assert_eq!(
        body_json(&response)["error"]["code"],
        "identity-session-cross-project"
    );
}

#[test]
fn identity_config_validates_unconfigured_and_invalid_honestly() {
    let (tmp, db, config) = fixture();
    let token = login_token(&config, &db);

    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/p1/identity/config",
    );
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    assert_eq!(body["identity_config"]["provider"], json!("okta"));
    assert_eq!(
        body["identity_config"]["issuer"],
        json!("https://example.okta.com")
    );
    assert_eq!(body["contract"], forge::api::API_CONTRACT_VERSION);

    let bare = tmp.path().join("bare");
    write_project(&bare, "bare", None);
    {
        let mut registry = Registry::open(&db).unwrap();
        registry.register(&bare, None).unwrap();
    }
    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/bare/identity/config",
    );
    assert_eq!(response.status, 404);
    assert_eq!(
        body_json(&response)["error"]["code"],
        "admin-identity-unconfigured"
    );

    let bad = tmp.path().join("bad-id");
    write_project(
        &bad,
        "bad-id",
        Some("identity:\n  provider: custom-idp\n  issuer: https://example.okta.com\n  client_id: forge-admin\n  redirect_uri: https://admin.example.com/oidc/callback\n  scopes:\n    - openid\n  admin_claim: groups\n  admin_values:\n    - forge-admins\n"),
    );
    {
        let mut registry = Registry::open(&db).unwrap();
        registry.register(&bad, None).unwrap();
    }
    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/bad-id/identity/config",
    );
    assert_eq!(response.status, 400);
    assert_eq!(body_json(&response)["error"]["code"], "identity-invalid");
}

#[test]
fn refused_reads_create_no_state_or_journal_rows() {
    let (tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let project = tmp.path().join("p1");
    let before_journal = journal_len(&db);
    let agents_dir = project.join(".forge/agents");
    let identity_dir = project.join(".forge/identity");

    for path in agent_identity_paths("..%2F..%2Fetc")
        .into_iter()
        .chain(agent_identity_paths("ghost-proj"))
        .chain(vec![
            "/v1/admin/projects/p1/agents/..".to_string(),
            "/v1/admin/projects/p1/identity/sessions/not-hex!!".to_string(),
            "/v1/admin/projects/p1/agents/no-such-session".to_string(),
            "/v1/admin/projects/p1/identity/sessions/ffffffffffffffff".to_string(),
        ])
    {
        let _ = get(&config, &db, &token, &path);
    }
    // Anonymous reads refuse before any Core call too.
    for path in agent_identity_paths("p1") {
        let _ = handle(
            &config,
            &db,
            &request("GET", &path, &config.frontend_origin),
            Utc::now(),
        );
    }

    assert_eq!(journal_len(&db), before_journal, "reads must not journal");
    assert!(!agents_dir.exists(), "reads must not create agent state");
    assert!(
        !identity_dir.exists(),
        "reads must not create identity state"
    );
}

#[test]
fn catalog_names_the_reads_and_keeps_writes_cli_only() {
    let rows = command_catalog::rows();
    assert_eq!(rows.len(), 234, "catalog conversion keeps row count");
    assert!(
        command_catalog::problems().is_empty(),
        "catalog problems: {:?}",
        command_catalog::problems()
    );
    let by_id: BTreeMap<&str, &command_catalog::CommandRow> =
        rows.iter().map(|row| (row.id.as_str(), row)).collect();
    for (id, route) in [
        (
            "agent.status",
            "GET /v1/admin/projects/{id}/agents/{session_id}",
        ),
        ("agent.list", "GET /v1/admin/projects/{id}/agents"),
        (
            "identity.validate-config",
            "GET /v1/admin/projects/{id}/identity/config",
        ),
        (
            "identity.session-list",
            "GET /v1/admin/projects/{id}/identity/sessions",
        ),
        (
            "identity.session-inspect",
            "GET /v1/admin/projects/{id}/identity/sessions/{session_id}",
        ),
    ] {
        let row = by_id.get(id).expect("read row exists");
        assert_eq!(row.availability, "web", "{id} is web");
        assert_eq!(row.route, Some(route), "{id} route");
        assert_eq!(row.risk, "read", "{id} risk");
    }
    // The revocation write converts to cli_only; the round-trip and every
    // lifecycle/secret/transport verb stays out of the browser.
    let terminate = by_id.get("identity.session-terminate").unwrap();
    assert_eq!(terminate.availability, "cli_only");
    assert_eq!(terminate.route, None);
    assert_eq!(
        by_id.get("identity.build-challenge").unwrap().availability,
        "not_yet_web"
    );
    for id in [
        "agent.start",
        "agent.pause",
        "agent.takeover",
        "agent.resume",
        "agent.restart",
        "agent.new-session",
        "agent.run-spec",
        "identity.setup",
        "identity.change-password",
        "identity.generate-password",
        "identity.status",
        "identity.complete-auth",
        "identity.session-validate",
        "identity.session-terminate",
        "api.serve",
        "web.serve",
        "mcp.serve",
        "portal.dashboard",
        "portal.view",
    ] {
        let row = by_id.get(id).expect("guarded row exists");
        assert_eq!(row.availability, "cli_only", "{id} stays CLI-only");
        assert_eq!(row.route, None, "{id} names no route");
    }
}

#[test]
fn frontend_workbench_card_uses_no_new_dependency_or_sink() {
    let html = index_html();
    let js = app_js();
    for token in [
        "wb-agent-identity-title",
        "agent-identity-error-summary",
        "agent-identity-agents",
        "agent-identity-agent-inspect",
        "agent-identity-agents-result",
        "agent-identity-sessions",
        "agent-identity-session-inspect",
        "agent-identity-sessions-result",
        "agent-identity-config",
        "agent-identity-config-result",
    ] {
        assert!(
            html.contains(token) || js.contains(token),
            "frontend token {token} missing"
        );
    }
    for route in ["/agents", "/identity/config", "/identity/sessions"] {
        assert!(js.contains(route), "frontend route {route} missing");
    }
    assert!(
        js.contains("agentIdentityListAgents"),
        "agent loader missing"
    );
    assert!(
        js.contains("agentIdentityReadConfig"),
        "config loader missing"
    );
    let start = js.find("Agent & identity").expect("card block");
    let block = &js[start..(start + 14000).min(js.len())];
    assert!(
        !block.contains("innerHTML"),
        "card block must not use innerHTML"
    );
}
