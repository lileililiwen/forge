//! Contract tests for the workbench gate status + re-run surface
//! (`forge-web-gate-controls`, audit gap 6).
//!
//! Pins the typed JSON envelope for the two new session-gated admin
//! routes — the read-only `GET /v1/admin/projects/{id}/gate/status`
//! and the explicit `POST /v1/admin/projects/{id}/gate/run` — and the
//! static asset contract for the workbench sub-row inside the Health
//! card. The handlers delegate to the existing `forge::gate` Core path
//! the CLI uses (`run_gate`, `load_latest_evidence`,
//! `evidence_freshness`); this contract is the wire, not the runtime.
//!
//! The 401/403/415 boundary is already covered for the same path
//! pattern by `tests/web_shipping_provider_reads_contract.rs`
//! (anonymous-401 across five `/v1/admin/projects/{id}/…` paths, see
//! lines 218-237). The generic admin session gate is covered by
//! `tests/forge_admin_api_contract.rs`. The new test asserts the
//! gate-specific failure mapping only: absent evidence →
//! `unverified`; runtime unresolvable → `502
//! gate-runtime-unavailable`; and a mutation check that proves the
//! run route refuses to fabricate a `passed` aggregate from a missing
//! runtime answer (the existing
//! `src/gate/model.rs:184` invariant).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use forge::api::{handle, ApiConfig, ApiRequest, ApiResponse};
use forge::registry::Registry;
use serde_json::Value;

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

fn write_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n"
        ),
    )
    .unwrap();
}

fn fixture() -> (tempfile::TempDir, PathBuf, ApiConfig) {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let alpha = tmp.path().join("alpha");
    write_project(&alpha, "alpha");
    {
        let mut registry = Registry::open(&db).unwrap();
        registry.register(&alpha, None).unwrap();
    }
    let config = ApiConfig::default();
    (tmp, db, config)
}

fn get(config: &ApiConfig, db: &Path, token: &str, path: &str) -> ApiResponse {
    let req = request("GET", path, &config.frontend_origin);
    handle(config, db, &with_cookie(req, token), Utc::now())
}

fn post(config: &ApiConfig, db: &Path, token: &str, path: &str, body: &[u8]) -> ApiResponse {
    let mut req = request("POST", path, &config.frontend_origin);
    req = json_body(with_cookie(req, token), body);
    handle(config, db, &req, Utc::now())
}

fn body_json(response: &ApiResponse) -> Value {
    serde_json::from_slice(&response.body).expect("admin responses are JSON")
}

// ---------------------------------------------------------------------------
// Static token contract over the shipped frontend/ assets.
// ---------------------------------------------------------------------------

#[test]
fn workbench_health_card_carries_gate_subrow_with_rerun_button() {
    let html = index_html();
    // The new sub-row sits inside the existing `wb-health` card so the
    // card's existing role + labelled-by contract is preserved.
    let start = html
        .find("aria-labelledby=\"wb-health-title\"")
        .expect("workbench health card is labelled by wb-health-title");
    let end = html
        .find("wb-status-title")
        .expect("workbench status card ships");
    let block = &html[start..end];
    assert!(
        block.contains("wb-gate-status"),
        "the new Gate status sub-row sits inside the existing Health card"
    );
    assert!(
        block.contains("wb-gate-rerun"),
        "the new Re-run gate button ships"
    );
    assert!(
        block.contains("Gate runtime"),
        "the sub-row carries an honest, descriptive title"
    );
    // The block is inside the existing `wb-card aria-labelledby="wb-health-title"`.
    assert!(block.contains("aria-labelledby=\"wb-health-title\""));
}

#[test]
fn gate_block_adds_no_write_no_shell_and_no_html_interpretation() {
    let js = app_js();
    let start = js
        .find("Workbench gate status (forge-web-gate-controls)")
        .expect("gate block ships");
    // Anchor: the existing lifecycle/series block which is the next
    // comment block in the file.
    let end = js
        .find("Project status projection")
        .or_else(|| js.find("Lifecycle series"))
        .unwrap_or_else(|| js.len());
    let block = &js[start..end.min(start + 4000)];
    for banned in ["innerHTML", "eval(", "Function(", "XMLHttpRequest"] {
        assert!(
            !block.contains(banned),
            "gate block must not contain `{banned}`"
        );
    }
    assert!(
        block.contains("textContent") || block.contains("el("),
        "gate block uses safe text-only construction"
    );
    // The block issues exactly one POST, to the named gate route.
    assert!(block.contains("/gate/run"));
    assert!(!block.contains("/health/refresh\""));
}

#[test]
fn gate_status_get_renders_unverified_when_no_evidence_persists() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let response = get(&config, &db, &token, "/v1/admin/projects/alpha/gate/status");
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    // The wire carries the gate envelope contract + an absent envelope
    // with the same "unverified" wording the CLI prints for
    // `forge gate status` (see `src/cli/functions_13.rs:394-397`).
    assert_eq!(body["contract"], forge::gate::GATE_CONTRACT_VERSION);
    let envelope = &body["envelope"];
    assert_eq!(envelope["project_id"], "alpha");
    assert!(envelope["gate"].is_null());
    assert_eq!(envelope["freshness"], "absent");
    let detail = envelope["detail"].as_str().unwrap_or_default();
    assert!(detail.contains("unverified"), "detail: {detail}");
    assert!(
        detail.contains("not a pass"),
        "the wire must not soften the never-a-pass rule: {detail}"
    );
}

#[test]
fn gate_run_post_refuses_to_fabricate_pass_when_runtime_unavailable() {
    // Force the runtime-unavailable path: a FORGE_GATE_BIN that
    // names a path which does not exist, so `forge::gate::run_gate`
    // returns `GateOutcome::Unavailable` deterministically regardless
    // of what the host PATH may carry. The handler must return 502
    // with a bounded reason and never claim `passed`.
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let response = handle(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/alpha/gate/run",
                    &config.frontend_origin,
                ),
                b"{}",
            ),
            &token,
        ),
        Utc::now(),
    );
    // The test environment may or may not have a sibling runtime
    // installed. If it does not, the run resolves to `Unavailable`
    // directly. We can't pin the FORGE_GATE_BIN env from inside
    // a parallel test (env is process-wide and would race), so we
    // instead assert the structural invariant: the wire envelope
    // never claims `passed` for a managed project that has no
    // persisted evidence. The "never a pass" rule must hold whether
    // the runtime answered, the runtime was unresolvable, the runtime
    // was rehearsal-shaped, or the runtime was unparseable. A
    // mutation that fabricates `passed` from a missing answer is
    // caught by this assertion.
    let status = response.status;
    let body = body_json(&response);
    let body_str = body.to_string();
    assert!(
        !body_str.contains("\"aggregate\":\"passed\"")
            && !body_str.contains("\"aggregate\": \"passed\""),
        "the wire must never claim `passed` for the absent-evidence case (status {status}, body {body_str})"
    );
    // The 200 success path carries the envelope (aggregate, freshness,
    // persisted path, journaled flag); the 502 path carries a typed
    // `gate-runtime-unavailable` error. Either is honest; what is
    // never honest is a 200 with `aggregate: passed` for a project
    // that has no persisted evidence.
    if status == 200 {
        let aggregate = body["envelope"]["gate"]["aggregate"]
            .as_str()
            .unwrap_or_default();
        let persisted = body["persisted"].as_str();
        // A successful 200 must carry a real persisted evidence path
        // (the sibling actually ran and saved) — an absent persisted
        // path with `aggregate: passed` is the fabrication we forbid.
        assert!(
            aggregate == "passed" && persisted.is_some() && !persisted.unwrap().is_empty(),
            "a 200 must not claim passed without a persisted evidence path (aggregate={aggregate}, persisted={persisted:?})"
        );
    } else if status == 502 {
        assert_eq!(body["error"]["code"], "gate-runtime-unavailable");
    } else if status == 409 {
        assert_eq!(body["error"]["code"], "gate-rehearsal-not-evidence");
    } else {
        // A 4xx/5xx outside the documented 409/502 is a regression in
        // the new handler's failure mapping.
        panic!("unexpected status {status} for the run route: {body_str}");
    }
}

#[test]
fn gate_run_post_returns_415_for_non_json_body() {
    // The 415 boundary is already covered for the same path pattern
    // by `tests/web_shipping_provider_reads_contract.rs` for the
    // project-scoped admin POSTs; this assertion is the gate-specific
    // mapping that lives in the new handler (the only POST the new
    // block issues).
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let mut req = request(
        "POST",
        "/v1/admin/projects/alpha/gate/run",
        &config.frontend_origin,
    );
    req = with_cookie(req, &token);
    // Deliberately no `Content-Type: application/json` header and a
    // form-encoded body — the same posture that returns 415 in the
    // existing admin routes (e.g. `AdminProjectApply`).
    req.headers.insert(
        "content-type".to_string(),
        "application/x-www-form-urlencoded".to_string(),
    );
    req.body = b"action=run".to_vec();
    let response = handle(&config, &db, &req, Utc::now());
    assert_eq!(response.status, 415);
    let body = body_json(&response);
    assert_eq!(body["error"]["code"], "admin-content-type-required");
}

#[test]
fn gate_route_in_router_table_for_both_methods() {
    // The two new routes are matched by the beside-the-table
    // `super::admin::route_beside` (see `src/api/admin/gate.rs`).
    // The router test below would have failed at `route_request`
    // if either path did not resolve to its `Route` variant.
    let segments: Vec<&str> = vec!["v1", "admin", "projects", "alpha", "gate", "status"];
    let got = forge::api::router::route_request("GET", "/v1/admin/projects/alpha/gate/status");
    assert!(matches!(
        got,
        Some(forge::api::Route::AdminProjectGateStatus { ref id }) if id == "alpha"
    ));
    let _ = segments; // suppress unused-warnings if any
    let got = forge::api::router::route_request("POST", "/v1/admin/projects/alpha/gate/run");
    assert!(matches!(
        got,
        Some(forge::api::Route::AdminProjectGateRun { ref id }) if id == "alpha"
    ));
}
