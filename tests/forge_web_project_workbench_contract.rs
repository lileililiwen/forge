//! Project workbench contract (`forge-web-project-workbench`).
//!
//! Pins the security boundary and workflow behavior of the typed,
//! session-gated single-project workbench routes
//! `GET /v1/admin/projects/{id}`, `GET …/plan` and `POST …/apply`:
//! anonymous requests get `401` with no workbench data, hostile origins get
//! `403` (including the deep-path CORS preflights), shell-metacharacter and
//! cross-path ids are refused as typed `400`s whose body never echoes the
//! input, unmanaged/observed-only projects get an honest `404`, plans are
//! provably side-effect-free, applies are confirm- and digest-bound,
//! idempotent replays never re-run the write, and no response ever
//! serializes the registered absolute filesystem path.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use chrono::Utc;
use forge::api::{handle, ApiConfig, ApiRequest};
use forge::registry::Registry;
use tempfile::tempdir;

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

/// Seed the global administrator and return a live session token.
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

/// Write a minimal schema-1 rust-web project with a stale pinned feature so
/// the upgrade plan has exactly one deterministic step.
fn write_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nfeatures:\n  auth: \"0.0.9\"\n"
        ),
    )
    .unwrap();
}

fn register_project(db: &Path, dir: &Path) {
    let mut registry = Registry::open(db).unwrap();
    registry.register(dir, None).unwrap();
}

fn body_json(response: &forge::api::ApiResponse) -> serde_json::Value {
    serde_json::from_slice(&response.body).expect("workbench routes are JSON-only")
}

#[test]
fn anonymous_requests_are_refused_before_any_workbench_data_leaks() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    forge::identity::global::setup(&db, "operator@example.test", "a-long-test-password").unwrap();
    let proj = dir.path().join("proj");
    write_project(&proj, "anon-proj");
    register_project(&db, &proj);

    let detail = handle(
        &config,
        &db,
        &request(
            "GET",
            "/v1/admin/projects/anon-proj",
            &config.frontend_origin,
        ),
        Utc::now(),
    );
    assert_eq!(detail.status, 401);
    let plan = handle(
        &config,
        &db,
        &request(
            "GET",
            "/v1/admin/projects/anon-proj/plan",
            &config.frontend_origin,
        ),
        Utc::now(),
    );
    assert_eq!(plan.status, 401);
    let apply = handle(
        &config,
        &db,
        &json_body(
            request(
                "POST",
                "/v1/admin/projects/anon-proj/apply",
                &config.frontend_origin,
            ),
            br#"{"confirm":true,"plan_digest":"deadbeef"}"#,
        ),
        Utc::now(),
    );
    assert_eq!(apply.status, 401);
    for response in [&detail, &plan, &apply] {
        let text = String::from_utf8_lossy(&response.body);
        assert!(!text.contains("manifest"), "401 body leaked data: {text}");
        assert!(
            !text.contains("plan_digest"),
            "401 body leaked a plan: {text}"
        );
    }
}

#[test]
fn hostile_origins_are_refused_including_the_deep_path_preflights() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);

    let hostile = "https://attacker.example";
    let detail = handle(
        &config,
        &db,
        &with_cookie(
            request("GET", "/v1/admin/projects/anything", hostile),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(detail.status, 403);
    for (method, path) in [
        ("OPTIONS", "/v1/admin/projects/anything"),
        ("OPTIONS", "/v1/admin/projects/anything/plan"),
        ("OPTIONS", "/v1/admin/projects/anything/apply"),
    ] {
        let preflight = handle(&config, &db, &request(method, path, hostile), Utc::now());
        assert_eq!(preflight.status, 403, "{method} {path}");
    }
    // The same preflight from the configured origin is allowed.
    let ok = handle(
        &config,
        &db,
        &request(
            "OPTIONS",
            "/v1/admin/projects/anything/apply",
            &config.frontend_origin,
        ),
        Utc::now(),
    );
    assert_eq!(ok.status, 204);
}

#[test]
fn shell_metacharacter_and_cross_path_ids_are_refused_without_echo_or_effect() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_project(&proj, "safe-proj");
    register_project(&db, &proj);

    let journal_before = Registry::open_read_only(&db)
        .unwrap()
        .journal_entries()
        .unwrap()
        .len();

    for id in [
        "safe-proj; rm -rf /",
        "$(whoami)",
        "evil|curl evil.test",
        "back`tick`",
        "UPPER-$(id)",
    ] {
        let response = handle(
            &config,
            &db,
            &with_cookie(
                request(
                    "GET",
                    &format!("/v1/admin/projects/{id}"),
                    &config.frontend_origin,
                ),
                &token,
            ),
            Utc::now(),
        );
        assert_eq!(response.status, 400, "id {id:?} must be a typed refusal");
        let body = String::from_utf8_lossy(&response.body).to_string();
        assert!(body.contains("not a valid identifier"), "{body}");
        for probe in ["rm", "$(", "|", "`", "whoami"] {
            assert!(
                !body.contains(probe),
                "the refused input leaked into the body: {body}"
            );
        }
        assert!(
            !body.contains(&proj.display().to_string()),
            "absolute path echoed: {body}"
        );
    }

    // Path traversal segments never reach the filesystem layer.
    for path in [
        "/v1/admin/projects/../../etc/passwd",
        "/v1/admin/projects/%2e%2e/secret/plan",
    ] {
        let response = handle(
            &config,
            &db,
            &with_cookie(request("GET", path, &config.frontend_origin), &token),
            Utc::now(),
        );
        assert!(
            response.status == 400 || response.status == 404,
            "traversal {path} must be refused, got {}",
            response.status
        );
    }

    // Nothing was planned, applied or journaled by the refusals.
    let journal_after = Registry::open_read_only(&db)
        .unwrap()
        .journal_entries()
        .unwrap()
        .len();
    assert_eq!(journal_before, journal_after);
    assert!(fs::read_to_string(proj.join("forge.yaml"))
        .unwrap()
        .contains("auth: \"0.0.9\""));
}

#[test]
fn unmanaged_and_observed_only_ids_get_the_honest_404_reason() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);

    let response = handle(
        &config,
        &db,
        &with_cookie(
            request(
                "GET",
                "/v1/admin/projects/ghost-project",
                &config.frontend_origin,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(response.status, 404);
    let body = body_json(&response);
    assert_eq!(body["error"]["code"], "workbench-unmanaged-project");
    assert_eq!(body["effect"], "none");
    let reason = body["error"]["message"].as_str().unwrap();
    assert!(reason.contains("not managed by this Forge registry"));
    assert!(reason.contains("observed-only"));
    assert!(!reason.contains("ghost-project"), "the id is not echoed");
}

#[test]
fn detail_is_a_read_only_path_free_projection_with_honest_dispositions() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_project(&proj, "wb-detail");
    register_project(&db, &proj);

    let response = handle(
        &config,
        &db,
        &with_cookie(
            request(
                "GET",
                "/v1/admin/projects/wb-detail",
                &config.frontend_origin,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    assert_eq!(body["contract"], "forge-project-workbench/0.1.0");
    assert_eq!(body["project_id"], "wb-detail");
    assert_eq!(body["management"], "managed");
    assert_eq!(body["manifest"]["id"], "wb-detail");
    assert_eq!(body["manifest"]["profile"], "rust-web");
    assert!(
        body["manifest"].get("path").is_none(),
        "manifest.path stripped"
    );
    let state = body["health"]["state"].as_str().unwrap();
    assert!(
        ["healthy", "stale", "issues", "unavailable"].contains(&state),
        "health state must be honest, got {state}"
    );
    let text = String::from_utf8_lossy(&response.body).to_string();
    assert!(
        !text.contains(&proj.canonicalize().unwrap().display().to_string()),
        "the registered absolute path leaked into the detail projection"
    );

    // Honest dispositions: the wired workflows name their typed routes; the
    // rest carry the catalog's reason and are explicitly not available.
    let workflows = body["workflows"].as_array().unwrap();
    let find = |id: &str| {
        workflows
            .iter()
            .find(|row| row["id"] == id)
            .expect("scoped workflow")
    };
    assert_eq!(find("inspect")["availability"], "web");
    assert_eq!(find("doctor")["availability"], "web");
    assert_eq!(find("upgrade")["availability"], "web");
    for cli_id in ["register", "new", "gate", "test"] {
        let row = find(cli_id);
        assert_eq!(row["available"], false, "{cli_id} must not look actionable");
        assert!(
            row["reason"].as_str().unwrap().chars().count() > 10,
            "{cli_id} must carry a plain-language reason"
        );
    }

    // Journal evidence: the register operation is visible with identity.
    let operations = body["operations"].as_array().unwrap();
    assert!(operations.iter().any(|row| row["kind"] == "register"));
}

#[test]
fn plan_is_side_effect_free_and_binds_confirmation_to_its_digest() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_project(&proj, "wb-plan");
    register_project(&db, &proj);
    let manifest_before = fs::read_to_string(proj.join("forge.yaml")).unwrap();
    let journal_before = Registry::open_read_only(&db)
        .unwrap()
        .journal_entries()
        .unwrap()
        .len();

    let response = handle(
        &config,
        &db,
        &with_cookie(
            request(
                "GET",
                "/v1/admin/projects/wb-plan/plan",
                &config.frontend_origin,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    assert_eq!(body["effect"], "none");
    assert_eq!(body["contract"], "forge-project-workbench/0.1.0");
    let digest = body["plan_digest"].as_str().unwrap();
    assert_eq!(digest.len(), 64);
    assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
    let steps = body["plan"]["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0]["feature"], "auth");
    assert_eq!(steps[0]["old_version"], "0.0.9");
    assert_eq!(steps[0]["new_version"], "0.1.0");
    assert!(body["confirmation"]["requires"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("plan_digest")));

    // No side effects at all: files, registry rows and journal unchanged.
    assert_eq!(
        fs::read_to_string(proj.join("forge.yaml")).unwrap(),
        manifest_before
    );
    let journal_after = Registry::open_read_only(&db)
        .unwrap()
        .journal_entries()
        .unwrap()
        .len();
    assert_eq!(journal_before, journal_after);

    // The optional feature query is accepted and never echoed raw.
    let mut scoped = request(
        "GET",
        "/v1/admin/projects/wb-plan/plan",
        &config.frontend_origin,
    );
    scoped.query = Some("feature=auth".to_string());
    let scoped = handle(&config, &db, &with_cookie(scoped, &token), Utc::now());
    assert_eq!(scoped.status, 200);

    let mut hostile = request(
        "GET",
        "/v1/admin/projects/wb-plan/plan",
        &config.frontend_origin,
    );
    hostile.query = Some("feature=auth%3B+rm%20-rf%20%2F".to_string());
    let hostile = handle(&config, &db, &with_cookie(hostile, &token), Utc::now());
    // The decoded value is only ever used as a catalog feature key: an
    // unknown one is a typed 400 refusal, never an execution.
    assert_eq!(hostile.status, 400);
    let text = String::from_utf8_lossy(&hostile.body).to_string();
    assert!(text.contains("not a known catalog feature"), "{text}");
    assert!(
        !text.contains("rm"),
        "the raw query leaked into the refusal: {text}"
    );
    assert_eq!(
        fs::read_to_string(proj.join("forge.yaml")).unwrap(),
        manifest_before
    );
}

#[test]
fn apply_requires_confirmation_and_a_matching_digest_before_any_write() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_project(&proj, "wb-refuse");
    register_project(&db, &proj);

    // Non-JSON bodies never reach the Core boundary.
    let wrong_type = handle(
        &config,
        &db,
        &with_cookie(
            request(
                "POST",
                "/v1/admin/projects/wb-refuse/apply",
                &config.frontend_origin,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(wrong_type.status, 415);

    // Missing confirm.
    let no_confirm = handle(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/wb-refuse/apply",
                    &config.frontend_origin,
                ),
                br#"{"plan_digest":"00"}"#,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(no_confirm.status, 409);
    assert_eq!(
        body_json(&no_confirm)["error"]["code"],
        "workbench-confirm-required"
    );

    // A forged/stale digest is refused and a refreshed plan is returned.
    let plan = handle(
        &config,
        &db,
        &with_cookie(
            request(
                "GET",
                "/v1/admin/projects/wb-refuse/plan",
                &config.frontend_origin,
            ),
            &token,
        ),
        Utc::now(),
    );
    let stale = handle(
        &config,
        &db,
        &with_cookie(
            json_body(
                request("POST", "/v1/admin/projects/wb-refuse/apply", &config.frontend_origin),
                br#"{"confirm":true,"plan_digest":"0000000000000000000000000000000000000000000000000000000000000000"}"#,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(stale.status, 409);
    let stale_body = body_json(&stale);
    assert_eq!(stale_body["error"]["code"], "workbench-plan-stale");
    assert_eq!(stale_body["effect"], "none");
    assert_eq!(
        stale_body["plan_digest"].as_str().unwrap(),
        body_json(&plan)["plan_digest"].as_str().unwrap(),
        "the refusal must carry the refreshed digest of the unchanged plan"
    );

    // Nothing was ever written by a refusal.
    assert!(fs::read_to_string(proj.join("forge.yaml"))
        .unwrap()
        .contains("auth: \"0.0.9\""));
    let operations = Registry::open_read_only(&db)
        .unwrap()
        .journal_entries()
        .unwrap();
    assert!(
        !operations
            .iter()
            .any(|entry| entry.kind == "workbench.upgrade.apply" && entry.state == "done"),
        "refusals must not journal a successful mutation"
    );
}

#[test]
fn confirmed_apply_writes_journals_and_replays_idempotently() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_project(&proj, "wb-apply");
    register_project(&db, &proj);

    let plan = handle(
        &config,
        &db,
        &with_cookie(
            request(
                "GET",
                "/v1/admin/projects/wb-apply/plan",
                &config.frontend_origin,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(plan.status, 200);
    let digest = body_json(&plan)["plan_digest"]
        .as_str()
        .unwrap()
        .to_string();

    let mut apply = request(
        "POST",
        "/v1/admin/projects/wb-apply/apply",
        &config.frontend_origin,
    );
    apply.idempotency_key = Some("wb-apply-key-1".to_string());
    let apply = handle(
        &config,
        &db,
        &with_cookie(
            json_body(
                apply,
                format!(r#"{{"confirm":true,"plan_digest":"{digest}"}}"#).as_bytes(),
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(apply.status, 202);
    let body = body_json(&apply);
    assert_eq!(body["contract"], "forge-project-workbench/0.1.0");
    assert_eq!(body["accepted"], true);
    assert!(
        body["operation_id"].is_number(),
        "a journaled operation id is returned"
    );

    // The write happened and is journaled under that operation identity.
    let manifest = fs::read_to_string(proj.join("forge.yaml")).unwrap();
    assert!(
        manifest.contains("0.1.0"),
        "the confirmed plan was applied: {manifest}"
    );
    let detail = handle(
        &config,
        &db,
        &with_cookie(
            request(
                "GET",
                "/v1/admin/projects/wb-apply",
                &config.frontend_origin,
            ),
            &token,
        ),
        Utc::now(),
    );
    let detail_body = body_json(&detail);
    assert!(detail_body["operations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| { row["kind"] == "workbench.upgrade.apply" && row["state"] == "done" }));
    let text = String::from_utf8_lossy(&detail.body).to_string();
    assert!(
        !text.contains(&proj.canonicalize().unwrap().display().to_string()),
        "journal detail leaked the absolute project path: {text}"
    );

    // Replaying the same idempotency key must not re-run the write.
    let mut replay = request(
        "POST",
        "/v1/admin/projects/wb-apply/apply",
        &config.frontend_origin,
    );
    replay.idempotency_key = Some("wb-apply-key-1".to_string());
    let replay = handle(
        &config,
        &db,
        &with_cookie(
            json_body(
                replay,
                format!(r#"{{"confirm":true,"plan_digest":"{digest}"}}"#).as_bytes(),
            ),
            &token,
        ),
        Utc::now(),
    );
    assert!(replay.status == 200 || replay.status == 202);
    assert_eq!(body_json(&replay)["replay"], true);

    // Re-applying with a fresh key after the project moved on: the new plan
    // is a no-op and still applies cleanly (no version drift).
    let plan2 = handle(
        &config,
        &db,
        &with_cookie(
            request(
                "GET",
                "/v1/admin/projects/wb-apply/plan",
                &config.frontend_origin,
            ),
            &token,
        ),
        Utc::now(),
    );
    let digest2 = body_json(&plan2)["plan_digest"]
        .as_str()
        .unwrap()
        .to_string();
    let mut again = request(
        "POST",
        "/v1/admin/projects/wb-apply/apply",
        &config.frontend_origin,
    );
    again.idempotency_key = Some("wb-apply-key-2".to_string());
    let again = handle(
        &config,
        &db,
        &with_cookie(
            json_body(
                again,
                format!(r#"{{"confirm":true,"plan_digest":"{digest2}"}}"#).as_bytes(),
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(
        again.status, 202,
        "re-applying the current plan is a clean no-op"
    );
}

#[test]
fn a_missing_project_directory_reports_honest_unavailable_states() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_project(&proj, "wb-gone");
    register_project(&db, &proj);
    fs::remove_dir_all(&proj).unwrap();

    let detail = handle(
        &config,
        &db,
        &with_cookie(
            request("GET", "/v1/admin/projects/wb-gone", &config.frontend_origin),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(detail.status, 200);
    let body = body_json(&detail);
    assert_eq!(body["health"]["state"], "unavailable");
    assert!(body["health"]["note"]
        .as_str()
        .unwrap()
        .contains("Nothing was changed"));
    let text = String::from_utf8_lossy(&detail.body).to_string();
    assert!(
        !text.contains(&proj.display().to_string()),
        "removed path echoed: {text}"
    );

    let plan = handle(
        &config,
        &db,
        &with_cookie(
            request(
                "GET",
                "/v1/admin/projects/wb-gone/plan",
                &config.frontend_origin,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(plan.status, 404);
    let plan_body = body_json(&plan);
    assert_eq!(plan_body["error"]["code"], "path-unavailable");
    assert_eq!(plan_body["effect"], "none");
    let text = String::from_utf8_lossy(&plan.body).to_string();
    assert!(
        !text.contains(&proj.display().to_string()),
        "path leaked into refusal: {text}"
    );
}

#[test]
fn failed_operations_surface_with_identity_state_and_redacted_detail() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_project(&proj, "wb-partial");
    register_project(&db, &proj);

    // A journaled partial failure (some stages done, the operation failed)
    // must stay visible with its identity, state and a path-free detail.
    let registry = Registry::open(&db).unwrap();
    registry
        .record_operation(
            "workbench.upgrade.apply",
            "wb-partial",
            "failed",
            "stage codemod failed for /home/operator/projects/wb-partial/src/main.rs",
        )
        .unwrap();

    let detail = handle(
        &config,
        &db,
        &with_cookie(
            request(
                "GET",
                "/v1/admin/projects/wb-partial",
                &config.frontend_origin,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(detail.status, 200);
    let body = body_json(&detail);
    let failed = body["operations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["kind"] == "workbench.upgrade.apply" && row["state"] == "failed")
        .expect("the failed operation is part of the evidence");
    assert!(
        failed["operation_id"].is_number(),
        "operation identity shown"
    );
    assert!(
        failed["started_at"].is_string(),
        "observation timestamp labeled"
    );
    let redacted = failed["detail"].as_str().unwrap();
    assert!(
        redacted.contains("stage codemod failed"),
        "logical reason kept"
    );
    assert!(
        !redacted.contains("/home") && !redacted.contains("operator"),
        "absolute path leaked into journal detail: {redacted}"
    );
    let text = String::from_utf8_lossy(&detail.body).to_string();
    assert!(
        !text.contains(&proj.canonicalize().unwrap().display().to_string()),
        "registered path leaked into the projection"
    );
}

#[test]
fn frontend_workbench_is_standalone_json_only_and_shell_free() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let app = fs::read_to_string(root.join("frontend/app.js")).unwrap();
    let index = fs::read_to_string(root.join("frontend/index.html")).unwrap();

    assert!(
        index.contains("id=\"workbench\""),
        "workbench section exists"
    );
    assert!(
        index.contains("id=\"wb-confirm\""),
        "explicit confirmation control"
    );
    assert!(
        app.contains("/v1/admin/projects/${encodeURIComponent(id)}"),
        "typed detail endpoint"
    );
    assert!(app.contains("/plan"), "typed plan endpoint");
    assert!(app.contains("/apply"), "typed apply endpoint");
    assert!(
        app.contains("plan_digest"),
        "confirmation binds to the reviewed digest"
    );
    // The workbench region renders structured state only: no HTML sinks, no
    // dynamic code execution — the browser sends only ids, digests and
    // catalog feature keys, never a path, shell text or injected command.
    let workbench_start = app
        .find("// ---- Project workbench")
        .expect("workbench section marker");
    let workbench_region = &app[workbench_start..];
    for forbidden in ["eval(", "new Function", "child_process", "innerHTML"] {
        assert!(
            !workbench_region.contains(forbidden),
            "unsafe sink in the workbench frontend: {forbidden}"
        );
    }
    assert!(
        !index.contains("<script>"),
        "no inline script in the workbench page"
    );
}
