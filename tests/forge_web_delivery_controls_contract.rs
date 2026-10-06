//! Delivery controls contract (`forge-web-delivery-controls`).
//!
//! These drive the real `/v1/admin/delivery*` handlers and pin the security
//! boundary of the confirmed share pipeline: anonymous requests get `401`
//! with no delivery data, hostile origins get `403` (including the deep-path
//! CORS preflights), every mutation is refused without an explicit
//! confirmation, a changed or stale digest is refused with `effect: none` and
//! provably no record/approval/artifact is written, shell-metacharacter and
//! traversal ids and operation keys are refused as typed `400`s whose body
//! never echoes the input, an unconfigured server-side publication target is
//! an honest typed prerequisite, an ambiguous `unknown` attempt is surfaced
//! and only a confirmed, digest-matched reconciliation clears it, and no
//! response ever serializes an absolute filesystem path — including the
//! publish report's `local-file:<path>` publisher label, which is reduced to
//! a safe kind. The provider seam (publish only writes after confirm plus a
//! matching digest, and is idempotent per operation key) is exercised
//! deterministically against a real temp target.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use forge::api::{handle, ApiConfig, ApiRequest, ApiResponse};
use forge::portfolio::share::PublicationStatus;
use forge::registry::{PublicationReservation, Registry};
use serde_json::{json, Value};
use tempfile::tempdir;

const CONTRACT: &str = "forge-web-delivery-controls/0.1.0";
const EMAIL: &str = "operator@example.test";
const PASSWORD: &str = "a-long-test-password";
const TARGET_ENV: &str = "FORGE_SHARE_PUBLISH_TARGET";
const SOURCE_URL: &str = "https://forge-delivery.example.com/app";

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

fn json_body(mut req: ApiRequest, value: &Value) -> ApiRequest {
    req.headers
        .insert("content-type".to_string(), "application/json".to_string());
    req.body = value.to_string().into_bytes();
    req
}

fn login_token(config: &ApiConfig, db: &Path) -> String {
    forge::identity::global::setup(db, EMAIL, PASSWORD).unwrap();
    let login = json_body(
        request("POST", "/v1/admin/session", &config.frontend_origin),
        &json!({"email": EMAIL, "password": PASSWORD}),
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

fn register_project(db: &Path, dir: &Path) {
    let mut registry = Registry::open(db).unwrap();
    registry.register(dir, None).unwrap();
}

fn get(config: &ApiConfig, db: &Path, token: &str, path: &str) -> ApiResponse {
    handle(
        config,
        db,
        &with_cookie(request("GET", path, &config.frontend_origin), token),
        Utc::now(),
    )
}

fn post(config: &ApiConfig, db: &Path, token: &str, path: &str, body: &Value) -> ApiResponse {
    handle(
        config,
        db,
        &with_cookie(
            json_body(request("POST", path, &config.frontend_origin), body),
            token,
        ),
        Utc::now(),
    )
}

fn body_json(response: &ApiResponse) -> Value {
    serde_json::from_slice(&response.body).expect("delivery routes are JSON-only")
}

fn leaks_path(response: &ApiResponse, root: &Path) -> bool {
    let text = String::from_utf8_lossy(&response.body).to_string();
    let candidates = [
        root.display().to_string(),
        root.canonicalize()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
    ];
    candidates
        .iter()
        .filter(|c| !c.is_empty())
        .any(|candidate| text.contains(candidate))
}

/// Advance the pipeline to an approved manifest and return its digest. The
/// allowlist add binds to the pre-write preview digest, the approval binds to
/// the post-write preview digest; both steps go through the real handlers.
fn approve_a_manifest(config: &ApiConfig, db: &Path, token: &str, project: &str) -> String {
    let before = body_json(&get(config, db, token, "/v1/admin/delivery/preview"));
    let digest0 = before["preview"]["manifest_sha256"]
        .as_str()
        .unwrap()
        .to_string();
    let set = post(
        config,
        db,
        token,
        &format!("/v1/admin/delivery/allowlist/{project}"),
        &json!({
            "confirm": true,
            "plan_digest": digest0,
            "title": format!("Delivery {project}"),
            "summary": "a shareable showcase project",
            "category": "demo",
            "source_url": SOURCE_URL,
            "visibility": "public",
        }),
    );
    assert_eq!(set.status, 200, "allowlist add must succeed: {set:?}");
    let after = body_json(&get(config, db, token, "/v1/admin/delivery/preview"));
    let digest1 = after["preview"]["manifest_sha256"]
        .as_str()
        .unwrap()
        .to_string();
    let approve = post(
        config,
        db,
        token,
        "/v1/admin/delivery/approve",
        &json!({"confirm": true, "plan_digest": digest1}),
    );
    assert_eq!(approve.status, 200, "approve must succeed: {approve:?}");
    digest1
}

#[test]
fn anonymous_requests_are_refused_before_any_delivery_data_leaks() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    forge::identity::global::setup(&db, EMAIL, PASSWORD).unwrap();
    let proj = dir.path().join("proj");
    write_project(&proj, "anon-proj");
    register_project(&db, &proj);

    for path in [
        "/v1/admin/delivery",
        "/v1/admin/delivery/preview",
        "/v1/admin/delivery/operation/any-key",
    ] {
        let response = handle(
            &config,
            &db,
            &request("GET", path, &config.frontend_origin),
            Utc::now(),
        );
        assert_eq!(response.status, 401, "GET {path}");
        let text = String::from_utf8_lossy(&response.body);
        assert!(
            !text.contains("manifest_sha256") && !text.contains("allowlist"),
            "401 leaked delivery data: {text}"
        );
    }

    // A mutating call is refused before the Core boundary runs: the JSON
    // content-type is required, so the session gate (401) is the one reached.
    for path in [
        "/v1/admin/delivery/approve",
        "/v1/admin/delivery/publish",
        "/v1/admin/delivery/reconcile",
        "/v1/admin/delivery/allowlist/anon-proj",
    ] {
        let response = handle(
            &config,
            &db,
            &json_body(
                request("POST", path, &config.frontend_origin),
                &json!({"confirm": true, "plan_digest": "x"}),
            ),
            Utc::now(),
        );
        assert_eq!(response.status, 401, "POST {path}");
    }
}

#[test]
fn hostile_origins_are_refused_including_the_deep_path_preflights() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let hostile = "https://attacker.example";

    let read = handle(
        &config,
        &db,
        &with_cookie(request("GET", "/v1/admin/delivery", hostile), &token),
        Utc::now(),
    );
    assert_eq!(read.status, 403);

    let write = handle(
        &config,
        &db,
        &with_cookie(
            json_body(
                request("POST", "/v1/admin/delivery/allowlist/anything", hostile),
                &json!({"confirm": true, "plan_digest": "x"}),
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(write.status, 403);

    for path in [
        "/v1/admin/delivery",
        "/v1/admin/delivery/preview",
        "/v1/admin/delivery/approve",
        "/v1/admin/delivery/publish",
        "/v1/admin/delivery/allowlist/anything",
        "/v1/admin/delivery/allowlist/anything/remove",
    ] {
        let preflight = handle(&config, &db, &request("OPTIONS", path, hostile), Utc::now());
        assert_eq!(preflight.status, 403, "OPTIONS {path}");
    }
    let ok = handle(
        &config,
        &db,
        &request(
            "OPTIONS",
            "/v1/admin/delivery/allowlist/anything/remove",
            &config.frontend_origin,
        ),
        Utc::now(),
    );
    assert_eq!(ok.status, 204, "allowed-origin preflight must succeed");
}

#[test]
fn unconfirmed_mutations_are_refused_with_no_effect() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_project(&proj, "dproj");
    register_project(&db, &proj);
    let digest = body_json(&get(&config, &db, &token, "/v1/admin/delivery/preview"))["preview"]
        ["manifest_sha256"]
        .as_str()
        .unwrap()
        .to_string();

    // confirm:false is refused, no record written.
    let refused = post(
        &config,
        &db,
        &token,
        "/v1/admin/delivery/allowlist/dproj",
        &json!({"confirm": false, "plan_digest": digest, "title": "T", "summary": "S", "category": "c", "source_url": SOURCE_URL}),
    );
    assert_eq!(refused.status, 409);
    assert_eq!(
        body_json(&refused)["error"]["code"],
        "delivery-confirm-required"
    );
    assert_eq!(body_json(&refused)["effect"], "none");
    assert_eq!(
        body_json(&get(&config, &db, &token, "/v1/admin/delivery"))["allowlist"]
            .as_array()
            .unwrap()
            .len(),
        0
    );

    // Unconfirmed approve is refused too.
    let ap = post(
        &config,
        &db,
        &token,
        "/v1/admin/delivery/approve",
        &json!({"confirm": false, "plan_digest": digest}),
    );
    assert_eq!(ap.status, 409);
    assert_eq!(body_json(&ap)["error"]["code"], "delivery-confirm-required");

    // Unconfirmed publish is refused before any target/approval check.
    let pu = post(
        &config,
        &db,
        &token,
        "/v1/admin/delivery/publish",
        &json!({"confirm": false, "plan_digest": digest, "operation_key": "op-1"}),
    );
    assert_eq!(pu.status, 409);
    assert_eq!(body_json(&pu)["error"]["code"], "delivery-confirm-required");
}

#[test]
fn stale_digest_is_refused_and_changes_nothing() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_project(&proj, "dproj");
    register_project(&db, &proj);

    // A digest that is not the current manifest (here a well-formed but wrong
    // hash) is refused with effect none and no record is written.
    let wrong = "0".repeat(64);
    let refused = post(
        &config,
        &db,
        &token,
        "/v1/admin/delivery/allowlist/dproj",
        &json!({"confirm": true, "plan_digest": wrong, "title": "T", "summary": "S", "category": "c", "source_url": SOURCE_URL}),
    );
    assert_eq!(refused.status, 409);
    assert_eq!(body_json(&refused)["error"]["code"], "delivery-plan-stale");
    assert_eq!(body_json(&refused)["effect"], "none");
    assert!(
        body_json(&refused).get("preview").is_some(),
        "the stale refusal carries a refreshed preview"
    );
    assert_eq!(
        body_json(&get(&config, &db, &token, "/v1/admin/delivery"))["allowlist"]
            .as_array()
            .unwrap()
            .len(),
        0,
        "no record written on a stale digest"
    );

    // Approving with a stale digest is refused too.
    let ap = post(
        &config,
        &db,
        &token,
        "/v1/admin/delivery/approve",
        &json!({"confirm": true, "plan_digest": wrong}),
    );
    assert_eq!(ap.status, 409);
    assert_eq!(body_json(&ap)["error"]["code"], "delivery-plan-stale");
    assert!(
        body_json(&get(&config, &db, &token, "/v1/admin/delivery"))["approval"].is_null(),
        "no approval row created on a stale digest"
    );
}

#[test]
fn invalid_ids_and_operation_keys_are_typed_refusals_that_never_echo_input() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let digest = "0".repeat(64);

    for bad in [
        "dproj; rm -rf /",
        "$(whoami)",
        "evil|curl evil.test",
        "back`tick`",
        "UPPER-$(id)",
    ] {
        // No '/' segment, so the id reaches the typed validator. require_managed
        // runs before the confirm/digest gate, so this is a 400 refusal.
        let response = post(
            &config,
            &db,
            &token,
            &format!("/v1/admin/delivery/allowlist/{bad}"),
            &json!({"confirm": true, "plan_digest": digest, "title": "T", "summary": "S", "category": "c", "source_url": SOURCE_URL}),
        );
        assert_eq!(response.status, 400, "id {bad:?} must be a typed refusal");
        let body = String::from_utf8_lossy(&response.body).to_string();
        for probe in ["rm", "$(", "|", "`", "whoami", "evil", "curl"] {
            assert!(!body.contains(probe), "the refused input leaked: {body}");
        }
    }

    // Traversal-shaped ids never resolve to a record and are refused.
    for path in [
        "/v1/admin/delivery/allowlist/../../etc/passwd",
        "/v1/admin/delivery/allowlist/%2e%2e/secret",
    ] {
        let response = post(
            &config,
            &db,
            &token,
            path,
            &json!({"confirm": true, "plan_digest": digest, "title": "T", "summary": "S", "category": "c", "source_url": SOURCE_URL}),
        );
        assert!(
            response.status == 400 || response.status == 404,
            "traversal {path} must be refused, got {}",
            response.status
        );
    }

    // A shell-metacharacter operation key is refused on publish before any
    // target lookup, and the input is never echoed.
    let pu = post(
        &config,
        &db,
        &token,
        "/v1/admin/delivery/publish",
        &json!({"confirm": true, "plan_digest": digest, "operation_key": "op; rm -rf /"}),
    );
    assert_eq!(pu.status, 400);
    assert_eq!(body_json(&pu)["error"]["code"], "delivery-invalid");
    let text = String::from_utf8_lossy(&pu.body).to_string();
    assert!(
        !text.contains("rm") && !text.contains(';'),
        "key echoed: {text}"
    );

    // A status read by a hostile key is refused as a typed 400, never a probe.
    let read = get(&config, &db, &token, "/v1/admin/delivery/operation/a|b");
    assert_eq!(read.status, 400);
    assert_eq!(body_json(&read)["error"]["code"], "delivery-invalid");
}

#[test]
fn empty_delivery_reports_honest_not_run_and_cli_only_states() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);

    let overview = body_json(&get(&config, &db, &token, "/v1/admin/delivery"));
    assert_eq!(overview["contract"], CONTRACT);
    assert_eq!(overview["live_probes"], false);
    assert!(overview["allowlist"].as_array().unwrap().is_empty());
    assert!(overview["approval"].is_null());
    assert!(overview["unreconciled"].is_null());
    assert!(overview["next_step"].is_string());

    // Repository/remote delivery stays honest CLI-only; the subprocess
    // adapter is never reachable from the browser.
    assert_eq!(overview["repository_operations"]["web"], false);
    assert_eq!(overview["adapter"]["web"], false);
    assert!(overview["repository_operations"]["note"].is_string());

    // The target block only reports configured-ness, never a path value.
    let configured = overview["target"]["configured"].as_bool();
    assert!(configured.is_some(), "target reports a boolean state");
    let _ = configured;

    // Provider matrix is non-live: every row is not-run, never a pass.
    let rows = overview["provider"]["rows"].as_array().unwrap();
    assert!(!rows.is_empty(), "provider matrix is listed");
    for row in rows {
        let status = row["status"].as_str().unwrap();
        assert!(
            status.contains("not-run") || status.contains("not_run"),
            "no provider is probed on page load, got {status}"
        );
    }

    // The routes self-describe exactly the eight typed endpoints.
    let routes = &overview["routes"];
    assert_eq!(routes["approve"], "POST /v1/admin/delivery/approve");
    assert_eq!(
        routes["operation"],
        "GET /v1/admin/delivery/operation/{key}"
    );

    // Preview is a side-effect-free plan.
    let preview = body_json(&get(&config, &db, &token, "/v1/admin/delivery/preview"));
    assert_eq!(preview["effect"], "none");
    assert!(preview["preview"]["manifest_sha256"].is_string());
    assert_eq!(preview["confirmation"]["requires"][0], "confirm");
}

#[test]
fn publish_writes_only_after_confirmation_and_a_matching_digest_and_is_idempotent() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    std::env::remove_var(TARGET_ENV);
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_project(&proj, "dproj");
    register_project(&db, &proj);

    let digest = approve_a_manifest(&config, &db, &token, "dproj");
    let target = dir.path().join("site").join("portfolio-manifest.json");

    // With no server-side target, publish is an honest typed prerequisite and
    // nothing is written.
    let prerequisite = post(
        &config,
        &db,
        &token,
        "/v1/admin/delivery/publish",
        &json!({"confirm": true, "plan_digest": digest, "operation_key": "deploy-1"}),
    );
    assert_eq!(prerequisite.status, 409);
    assert_eq!(
        body_json(&prerequisite)["error"]["code"],
        "delivery-prerequisite"
    );
    assert!(!target.exists(), "no artifact written without a target");

    std::env::set_var(TARGET_ENV, target.to_str().unwrap());

    // Publishing a digest that is not the newest approval is refused with no
    // artifact written.
    let wrong = post(
        &config,
        &db,
        &token,
        "/v1/admin/delivery/publish",
        &json!({"confirm": true, "plan_digest": "f".repeat(64), "operation_key": "deploy-1"}),
    );
    assert_eq!(wrong.status, 409);
    assert_eq!(body_json(&wrong)["error"]["code"], "delivery-plan-stale");
    assert!(
        !target.exists(),
        "no artifact written on a non-approved digest"
    );

    // The confirmed publish with the approved digest writes the artifact and
    // reports the honest local outcome — with no server path in the body.
    let ok = post(
        &config,
        &db,
        &token,
        "/v1/admin/delivery/publish",
        &json!({"confirm": true, "plan_digest": digest, "operation_key": "deploy-1"}),
    );
    assert_eq!(ok.status, 200, "publish must succeed: {ok:?}");
    let published = body_json(&ok);
    assert_eq!(published["accepted"], true);
    assert_eq!(published["publication"]["status"], "published");
    assert_eq!(published["publication"]["operation_key"], "deploy-1");
    // The publisher is reported by kind, never by absolute path.
    assert_eq!(published["publication"]["publisher"], "local-file export");
    assert!(
        target.exists(),
        "the approved manifest landed on the target"
    );
    let artifact = fs::read_to_string(&target).unwrap();
    assert!(
        artifact.contains("dproj"),
        "the artifact carries the approved record"
    );
    assert!(
        !leaks_path(&ok, dir.path()),
        "publish response leaked an absolute path"
    );

    // A retry of the same operation key reconciles instead of double-writing.
    let before = fs::read_to_string(&target).unwrap();
    let retry = post(
        &config,
        &db,
        &token,
        "/v1/admin/delivery/publish",
        &json!({"confirm": true, "plan_digest": digest, "operation_key": "deploy-1"}),
    );
    assert_eq!(retry.status, 200, "same-key retry resolves: {retry:?}");
    assert_eq!(
        body_json(&retry)["publication"]["already_present"],
        true,
        "a retry does not publish a second time"
    );
    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        before,
        "the artifact is byte-for-byte unchanged on retry"
    );

    // Status by operation key reads the journaled attempt, no dispatch.
    let status = body_json(&get(
        &config,
        &db,
        &token,
        "/v1/admin/delivery/operation/deploy-1",
    ));
    assert_eq!(status["publication"]["status"], "published");
    assert!(status["guidance"].is_string());
    assert!(!leaks_path(
        &get(
            &config,
            &db,
            &token,
            "/v1/admin/delivery/operation/deploy-1"
        ),
        dir.path()
    ));

    std::env::remove_var(TARGET_ENV);
}

#[test]
fn an_unknown_attempt_is_surfaced_and_only_a_confirmed_digest_matched_reconciliation_clears_it() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    std::env::remove_var(TARGET_ENV);
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_project(&proj, "dproj");
    register_project(&db, &proj);

    let digest = approve_a_manifest(&config, &db, &token, "dproj");

    // Force an ambiguous (`unknown`) publication attempt deterministically at
    // the typed seam: reserve one attempt against the approved manifest, then
    // record its outcome as `unknown` — exactly the state a provider that
    // timed out mid-publish leaves behind. Nothing external happened.
    let registry = Registry::open(&db).unwrap();
    let approval = registry.share_latest_approval().unwrap().unwrap();
    let reservation = registry
        .share_reserve_publication(
            "ambiguous-1",
            approval.revision,
            &approval.manifest_sha256,
            "unused://target",
            "tester",
            "2020-01-01T00:00:00Z",
        )
        .unwrap();
    let publication_id = match reservation {
        PublicationReservation::Reserved { publication_id, .. } => publication_id,
        PublicationReservation::Reused { publication_id, .. } => publication_id,
    };
    registry
        .share_finish_publication(
            publication_id,
            PublicationStatus::Unknown,
            None,
            Some("provider-timeout"),
        )
        .unwrap();

    // The overview honestly surfaces the unreconciled attempt with its digest.
    let overview = body_json(&get(&config, &db, &token, "/v1/admin/delivery"));
    assert_eq!(overview["unreconciled"]["operation_key"], "ambiguous-1");
    assert_eq!(overview["unreconciled"]["status"], "unknown");
    let attempt_digest = overview["unreconciled"]["manifest_sha256"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(attempt_digest, digest);

    // Reconciling without confirmation is refused; the attempt stays unknown.
    let unconfirmed = post(
        &config,
        &db,
        &token,
        "/v1/admin/delivery/reconcile",
        &json!({"confirm": false, "publication_id": publication_id, "status": "failed", "plan_digest": attempt_digest}),
    );
    assert_eq!(unconfirmed.status, 409);
    assert_eq!(
        body_json(&unconfirmed)["error"]["code"],
        "delivery-confirm-required"
    );

    // Reconciling with the wrong digest is refused and changes nothing.
    let mismatch = post(
        &config,
        &db,
        &token,
        "/v1/admin/delivery/reconcile",
        &json!({"confirm": true, "publication_id": publication_id, "status": "failed", "plan_digest": "a".repeat(64)}),
    );
    assert_eq!(mismatch.status, 409);
    assert_eq!(body_json(&mismatch)["error"]["code"], "delivery-plan-stale");
    assert_eq!(
        body_json(&get(&config, &db, &token, "/v1/admin/delivery"))["unreconciled"]["status"],
        "unknown",
        "a refused reconciliation leaves the attempt unknown"
    );

    // `unknown` is never a reconciliation outcome.
    let bad_status = post(
        &config,
        &db,
        &token,
        "/v1/admin/delivery/reconcile",
        &json!({"confirm": true, "publication_id": publication_id, "status": "unknown", "plan_digest": attempt_digest}),
    );
    assert_eq!(bad_status.status, 400);
    assert_eq!(body_json(&bad_status)["error"]["code"], "delivery-invalid");

    // The confirmed, digest-matched reconciliation records the operator's
    // observed outcome and clears the unreconciled gate.
    let fixed = post(
        &config,
        &db,
        &token,
        "/v1/admin/delivery/reconcile",
        &json!({"confirm": true, "publication_id": publication_id, "status": "failed", "plan_digest": attempt_digest}),
    );
    assert_eq!(fixed.status, 200, "reconciliation must record: {fixed:?}");
    assert_eq!(body_json(&fixed)["effect"], "forge-owned-write");
    assert_eq!(body_json(&fixed)["publication"]["status"], "failed");
    assert!(
        body_json(&get(&config, &db, &token, "/v1/admin/delivery"))["unreconciled"].is_null(),
        "reconciling clears the unknown gate"
    );

    // A reconciliation must echo no absolute path.
    assert!(!leaks_path(&fixed, dir.path()));
}

#[test]
fn withdrawal_of_an_absent_record_is_honest_and_no_path_ever_leaks() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_project(&proj, "dproj");
    register_project(&db, &proj);

    let digest = body_json(&get(&config, &db, &token, "/v1/admin/delivery/preview"))["preview"]
        ["manifest_sha256"]
        .as_str()
        .unwrap()
        .to_string();

    // Withdrawing a record that does not exist reports `removed: false` with
    // effect none rather than faking a deletion.
    let removed = post(
        &config,
        &db,
        &token,
        "/v1/admin/delivery/allowlist/dproj/remove",
        &json!({"confirm": true, "plan_digest": digest}),
    );
    assert_eq!(removed.status, 200);
    let body = body_json(&removed);
    assert_eq!(body["removed"], false);
    assert_eq!(body["effect"], "none");

    // The overview never serializes the registered project path.
    let overview = get(&config, &db, &token, "/v1/admin/delivery");
    assert!(!leaks_path(&overview, &proj));
}

#[test]
fn frontend_delivery_is_standalone_json_only_and_shell_free() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let app = fs::read_to_string(root.join("frontend/app.js")).unwrap();
    let index = fs::read_to_string(root.join("frontend/index.html")).unwrap();

    assert!(index.contains("id=\"delivery\""), "delivery section exists");
    assert!(
        index.contains("id=\"delivery-publish\""),
        "typed publish confirmation control exists"
    );
    assert!(
        index.contains("id=\"delivery-approve\""),
        "typed approve control exists"
    );

    for endpoint in [
        "/v1/admin/delivery",
        "/v1/admin/delivery/approve",
        "/v1/admin/delivery/publish",
        "/v1/admin/delivery/reconcile",
        "/v1/admin/delivery/operation/",
        "/v1/admin/delivery/allowlist/",
    ] {
        assert!(
            app.contains(endpoint),
            "app.js must call the typed {endpoint} endpoint"
        );
    }
    // Ids and keys are sent percent-encoded, never interpolated into a shell.
    assert!(
        app.contains("encodeURIComponent(id)") || app.contains("encodeURIComponent(key)"),
        "delivery ids and keys must be percent-encoded"
    );

    let start = app
        .find("// ---- Delivery controls")
        .expect("delivery section marker");
    let end = app
        .find("async function dashboardPage")
        .expect("dashboard function");
    let region = &app[start..end];
    for forbidden in [
        "eval(",
        "new Function",
        "child_process",
        "innerHTML",
        "sh -c",
        "localStorage",
        "sessionStorage",
    ] {
        assert!(
            !region.contains(forbidden),
            "unsafe sink in the delivery frontend: {forbidden}"
        );
    }
    assert!(!index.contains("<script>"), "no inline script on the page");
}
