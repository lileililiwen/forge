//! Read-only project status and fleet readiness contract
//! (`forge-web-project-status`).
//!
//! Pins the security boundary and projection behavior of the two
//! session-gated, JSON-only reads `GET /v1/admin/projects/{id}/status` and
//! `GET /v1/admin/status`: anonymous requests get `401` with no status data,
//! hostile origins get `403`, hostile/path-bearing ids are refused as typed
//! `400`s whose body never echoes the input, valid-but-unmanaged ids get an
//! honest `404`, an unreadable root reduces to `unavailable` with safe
//! path-free reasons, a genuinely healthy project reads `healthy`, a real
//! missing build definition reads `issues` (never `healthy`), the fleet counts
//! sum to the registry total, and neither route ever serializes the registered
//! absolute path or a credential. Both routes are also proven read-only: the
//! project files and registry journal are byte-identical before and after.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use chrono::Utc;
use forge::api::{handle, ApiConfig, ApiRequest};
use forge::registry::Registry;
use tempfile::tempdir;

const PROJECT_CONTRACT: &str = "forge-project-status/0.1.0";

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
    let login = json_body(
        request("POST", "/v1/admin/session", &config.frontend_origin),
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
    serde_json::from_slice(&response.body).expect("status routes are JSON-only")
}

fn register_project(db: &Path, dir: &Path) {
    let mut registry = Registry::open(db).unwrap();
    registry.register(dir, None).unwrap();
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git is available");
    assert!(status.success(), "git {args:?} failed");
}

/// A minimal schema-1 rust-web project that lacks its build definition, so
/// doctor and checker report real `issues`.
fn write_broken_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: Broken {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nfeatures:\n  auth: \"0.0.9\"\n"
        ),
    )
    .unwrap();
}

/// A rust-web project with every L1 evidence marker, a git `origin` remote and
/// a commit, so doctor and checker are genuinely clean (never a warning).
fn write_healthy_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir.join(".github/workflows")).unwrap();
    fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: Healthy {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nfeatures: {{}}\n"
        ),
    )
    .unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"healthy\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(dir.join("README.md"), "# Healthy project\n").unwrap();
    fs::write(dir.join("Dockerfile"), "FROM scratch\n").unwrap();
    fs::write(
        dir.join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs: {}\n",
    )
    .unwrap();
    fs::write(dir.join("driftwatch.yaml"), "version: 1\n").unwrap();
    git(dir, &["init", "-q"]);
    git(dir, &["config", "user.email", "test@example.test"]);
    git(dir, &["config", "user.name", "Forge Test"]);
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", "init"]);
    git(
        dir,
        &[
            "remote",
            "add",
            "origin",
            "https://example.test/healthy.git",
        ],
    );
}

#[test]
fn anonymous_requests_are_refused_before_any_status_data_leaks() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let proj = dir.path().join("proj");
    write_healthy_project(&proj, "anon-status");
    register_project(&db, &proj);

    for path in ["/v1/admin/projects/anon-status/status", "/v1/admin/status"] {
        let response = handle(
            &config,
            &db,
            &request("GET", path, &config.frontend_origin),
            Utc::now(),
        );
        assert_eq!(response.status, 401, "{path}");
        let body = body_json(&response);
        assert_eq!(body["error"]["code"], "api-unauthorized", "{path}");
        for leaked in ["state", "checks", "counts", "projects"] {
            assert!(body.get(leaked).is_none(), "{path} leaked `{leaked}`");
        }
    }
}

#[test]
fn hostile_origins_are_rejected_with_a_valid_session() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let hostile = "https://attacker.example";

    for path in ["/v1/admin/projects/anything/status", "/v1/admin/status"] {
        let response = handle(
            &config,
            &db,
            &with_cookie(request("GET", path, hostile), &token),
            Utc::now(),
        );
        assert_eq!(response.status, 403, "{path}");
        assert_eq!(
            body_json(&response)["error"]["code"],
            "admin-origin-rejected"
        );
    }

    // The matching preflights stay origin-gated and the configured origin is
    // allowed (the existing wildcard arms already cover them).
    let ok = handle(
        &config,
        &db,
        &request(
            "OPTIONS",
            "/v1/admin/projects/anything/status",
            &config.frontend_origin,
        ),
        Utc::now(),
    );
    assert_eq!(ok.status, 204);
}

#[test]
fn hostile_ids_are_refused_without_echo_or_filesystem_access() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_broken_project(&proj, "safe-status");
    register_project(&db, &proj);

    for id in [
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
                    &format!("/v1/admin/projects/{id}/status"),
                    &config.frontend_origin,
                ),
                &token,
            ),
            Utc::now(),
        );
        assert_eq!(response.status, 400, "id {id:?} must be a typed refusal");
        let text = String::from_utf8_lossy(&response.body).to_string();
        assert!(text.contains("not a valid identifier"), "{text}");
        for probe in ["$(", "|", "`", "whoami"] {
            assert!(!text.contains(probe), "refused input leaked: {text}");
        }
        assert!(
            !text.contains(&proj.display().to_string()),
            "absolute path echoed: {text}"
        );
    }

    // A slash-bearing id can never match the five-segment route; it is still
    // a safe, non-echoing refusal (400 from the validator or 404 from the
    // router) and never reaches the filesystem layer.
    for id in ["safe-status; rm -rf /", "../../etc/passwd"] {
        let response = handle(
            &config,
            &db,
            &with_cookie(
                request(
                    "GET",
                    &format!("/v1/admin/projects/{id}/status"),
                    &config.frontend_origin,
                ),
                &token,
            ),
            Utc::now(),
        );
        assert!(
            response.status == 400 || response.status == 404,
            "path-bearing id {id:?} must be refused, got {}",
            response.status
        );
        let text = String::from_utf8_lossy(&response.body).to_string();
        assert!(
            !text.contains(&proj.display().to_string()),
            "absolute path echoed: {text}"
        );
        assert!(
            !text.contains("passwd"),
            "path-bearing input echoed: {text}"
        );
    }
}

#[test]
fn unmanaged_ids_get_the_honest_404_without_echo() {
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
                "/v1/admin/projects/ghost-project/status",
                &config.frontend_origin,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(response.status, 404);
    let body = body_json(&response);
    assert_eq!(body["contract"], PROJECT_CONTRACT);
    assert_eq!(body["error"]["code"], "project-status-unmanaged-project");
    assert_eq!(body["effect"], "none");
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("not managed by this Forge registry"));
    assert!(!String::from_utf8_lossy(&response.body).contains("ghost-project"));
}

#[test]
fn a_healthy_project_reads_healthy_on_every_check() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_healthy_project(&proj, "healthy-status");
    register_project(&db, &proj);

    let response = handle(
        &config,
        &db,
        &with_cookie(
            request(
                "GET",
                "/v1/admin/projects/healthy-status/status",
                &config.frontend_origin,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    assert_eq!(body["contract"], PROJECT_CONTRACT);
    assert_eq!(body["project_id"], "healthy-status");
    assert_eq!(body["management"], "managed");
    assert_eq!(body["live"], false);
    assert_eq!(body["state"], "healthy");

    let checks = body["checks"].as_array().unwrap();
    assert_eq!(checks.len(), 3);
    for (check, id) in checks.iter().zip(["doctor", "check", "readiness"]) {
        assert_eq!(check["id"], id);
        assert_eq!(check["state"], "healthy", "check {id}: {check}");
        assert_eq!(check["reason"], "");
    }

    // No absolute path or credential-shaped text is serialized.
    let text = String::from_utf8_lossy(&response.body);
    assert!(!text.contains(&proj.display().to_string()), "path leaked");
    assert!(!text.contains("ghp_"));
    assert!(!text.contains("driftwatch-"));
}

#[test]
fn a_missing_build_definition_reads_issues_never_healthy() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_broken_project(&proj, "broken-status");
    register_project(&db, &proj);

    let response = handle(
        &config,
        &db,
        &with_cookie(
            request(
                "GET",
                "/v1/admin/projects/broken-status/status",
                &config.frontend_origin,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    assert_eq!(body["state"], "issues");
    assert_eq!(body["checks"][0]["id"], "doctor");
    assert_eq!(body["checks"][0]["state"], "issues");
    assert_eq!(body["checks"][1]["id"], "check");
    assert_eq!(body["checks"][1]["state"], "issues");

    let text = String::from_utf8_lossy(&response.body);
    assert!(!text.contains(&proj.display().to_string()), "path leaked");
}

#[test]
fn an_unreadable_root_reduces_to_unavailable_with_path_free_reasons() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_healthy_project(&proj, "gone-status");
    register_project(&db, &proj);
    fs::remove_dir_all(&proj).unwrap();

    let response = handle(
        &config,
        &db,
        &with_cookie(
            request(
                "GET",
                "/v1/admin/projects/gone-status/status",
                &config.frontend_origin,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    assert_eq!(body["state"], "unavailable");
    let checks = body["checks"].as_array().unwrap();
    assert_eq!(checks.len(), 3);
    for check in checks {
        assert_eq!(check["state"], "unavailable");
        let reason = check["reason"].as_str().unwrap();
        assert!(
            !reason.is_empty(),
            "an unavailable check must carry a reason"
        );
        assert!(
            !reason.contains(&proj.display().to_string()),
            "reason leaked the path: {reason}"
        );
    }
    assert!(!String::from_utf8_lossy(&response.body).contains(&proj.display().to_string()));
}

#[test]
fn the_fleet_summary_counts_every_registered_project() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let healthy = dir.path().join("healthy");
    let broken = dir.path().join("broken");
    write_healthy_project(&healthy, "fleet-healthy");
    write_broken_project(&broken, "fleet-broken");
    register_project(&db, &healthy);
    register_project(&db, &broken);

    let response = handle(
        &config,
        &db,
        &with_cookie(
            request("GET", "/v1/admin/status", &config.frontend_origin),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    assert_eq!(body["contract"], PROJECT_CONTRACT);
    assert_eq!(body["live"], false);
    assert_eq!(body["total"], 2);
    assert!(body.get("truncated").is_none());

    let counts = &body["counts"];
    let sum = counts["healthy"].as_u64().unwrap()
        + counts["issues"].as_u64().unwrap()
        + counts["stale"].as_u64().unwrap()
        + counts["unavailable"].as_u64().unwrap();
    assert_eq!(
        sum,
        body["total"].as_u64().unwrap(),
        "counts must sum to total"
    );
    assert_eq!(counts["healthy"], 1);
    assert_eq!(counts["issues"], 1);

    let projects = body["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 2);
    let stored: Vec<(&str, &str)> = projects
        .iter()
        .map(|entry| {
            (
                entry["project_id"].as_str().unwrap(),
                entry["state"].as_str().unwrap(),
            )
        })
        .collect();
    assert!(
        stored.contains(&("fleet-healthy", "healthy")),
        "the healthy project must be counted healthy: {stored:?}"
    );

    let text = String::from_utf8_lossy(&response.body);
    assert!(!text.contains(&healthy.display().to_string()));
    assert!(!text.contains(&broken.display().to_string()));
}

#[test]
fn both_routes_are_read_only() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_healthy_project(&proj, "readonly-status");
    register_project(&db, &proj);

    let manifest_before = fs::read(proj.join("forge.yaml")).unwrap();
    let journal_before = Registry::open_read_only(&db)
        .unwrap()
        .journal_entries()
        .unwrap()
        .len();

    for path in [
        "/v1/admin/projects/readonly-status/status",
        "/v1/admin/status",
    ] {
        let response = handle(
            &config,
            &db,
            &with_cookie(request("GET", path, &config.frontend_origin), &token),
            Utc::now(),
        );
        assert_eq!(response.status, 200, "{path}");
    }

    let manifest_after = fs::read(proj.join("forge.yaml")).unwrap();
    assert_eq!(manifest_before, manifest_after, "manifest was rewritten");
    let journal_after = Registry::open_read_only(&db)
        .unwrap()
        .journal_entries()
        .unwrap()
        .len();
    assert_eq!(journal_before, journal_after, "a journal row was written");
}

#[test]
fn the_catalog_and_frontend_surface_both_routes() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);

    let response = handle(
        &config,
        &db,
        &with_cookie(
            request("GET", "/v1/admin/commands", &config.frontend_origin),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    let row = |id: &str| {
        body["commands"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == id)
            .cloned()
            .unwrap_or_else(|| panic!("catalog is missing `{id}`"))
    };
    let check = row("check");
    assert_eq!(check["availability"], "web");
    assert_eq!(check["route"], "GET /v1/admin/projects/{id}/status");
    let fleet = row("fleet.status");
    assert_eq!(fleet["availability"], "web");
    assert_eq!(fleet["route"], "GET /v1/admin/status");

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let index = fs::read_to_string(root.join("frontend/index.html")).unwrap();
    let app = fs::read_to_string(root.join("frontend/app.js")).unwrap();
    for id in [
        "wb-status",
        "fleet-readiness",
        "fleet-healthy",
        "fleet-issues",
        "fleet-stale",
        "fleet-unavailable",
        "fleet-total",
    ] {
        assert!(index.contains(&format!("id=\"{id}\"")), "missing #{id}");
    }
    assert!(
        app.contains("/v1/admin/status"),
        "app.js must fetch the fleet tile"
    );
    assert!(
        app.contains("/status`"),
        "app.js must fetch the per-project status endpoint"
    );
    assert!(!index.contains("<script>"), "no inline script");

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
}
