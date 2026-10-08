//! Contract tests for the authenticated CLI command catalog served by
//! `GET /v1/admin/commands` (`forge-web-command-catalog`).
//!
//! These drive the real handler: the global-session gate, the exhaustive
//! Clap-tree coverage, the truthful state/reason tuples, and the absolute
//! absence of any shell/eval invocation surface. The catalog is static
//! descriptive metadata; user-supplied strings must never influence it.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use chrono::Utc;
use forge::api::{handle, ApiConfig, ApiRequest};
use serde_json::Value;
use tempfile::TempDir;

const EMAIL: &str = "catalog-operator@example.test";
const PASSWORD: &str = "a-long-test-password";
const CATALOG_CONTRACT: &str = "forge-command-catalog/0.1.0";

/// Every top-level command name rendered by `forge --help`, asserted to be
/// a catalog row. The design's verification oracle calls this out
/// explicitly, transport and terminal commands included.
const TOP_LEVEL_COMMANDS: [&str; 48] = [
    "list",
    "inspect",
    "register",
    "import",
    "graduation",
    "profile",
    "kit",
    "new",
    "doctor",
    "check",
    "feature",
    "upgrade",
    "spec",
    "agent",
    "test",
    "commit",
    "push",
    "mcp",
    "mirror",
    "docs",
    "release",
    "deploy",
    "publish",
    "component",
    "ui-pattern",
    "intent",
    "procedure",
    "identity",
    "analytics",
    "api",
    "web",
    "portal",
    "portfolio",
    "readiness",
    "provider",
    "governance",
    "fleet",
    "project",
    "gate",
    "contract",
    "inventory",
    "standard",
    "remediate",
    "describe",
    "classify",
    "delivery",
    "studio",
    "help",
];

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

fn with_session(request: &mut ApiRequest, token: &str) {
    request
        .cookies
        .insert("forge_admin_session".to_string(), token.to_string());
}

fn setup_admin(db: &Path) {
    forge::identity::global::setup(db, EMAIL, PASSWORD).unwrap();
}

fn login_token(config: &ApiConfig, db: &Path) -> String {
    let mut login = request("POST", "/v1/admin/session", &config.frontend_origin);
    login
        .headers
        .insert("content-type".to_string(), "application/json".to_string());
    login.body = json_body(&login_body());
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

fn login_body() -> serde_json::Value {
    serde_json::json!({ "email": EMAIL, "password": PASSWORD })
}

fn json_body(value: &serde_json::Value) -> Vec<u8> {
    value.to_string().into_bytes()
}

fn body_json(response: &forge::api::ApiResponse) -> Value {
    serde_json::from_slice(&response.body).expect("admin responses are JSON")
}

#[test]
fn anonymous_requests_are_rejected_before_any_catalog_data() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("registry.db");
    setup_admin(&db);
    let config = ApiConfig::default();

    let response = handle(
        &config,
        &db,
        &request("GET", "/v1/admin/commands", &config.frontend_origin),
        Utc::now(),
    );
    assert_eq!(response.status, 401);
    let body = String::from_utf8_lossy(&response.body);
    assert!(body.contains("api-unauthorized"));
    // No catalog data may leak into an unauthenticated response.
    assert!(!body.contains("cli_invocation"));
    assert!(!body.contains("forge publish"));
}

#[test]
fn hostile_origins_are_rejected_even_with_a_valid_session() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("registry.db");
    setup_admin(&db);
    let config = ApiConfig::default();
    let token = login_token(&config, &db);

    let mut attacker = request("GET", "/v1/admin/commands", "https://evil.example");
    with_session(&mut attacker, &token);
    let response = handle(&config, &db, &attacker, Utc::now());
    assert_eq!(response.status, 403);
    assert!(body_json(&response)
        .pointer("/error/code")
        .and_then(Value::as_str)
        .is_some_and(|code| code == "admin-origin-rejected"));
}

#[test]
fn authenticated_catalog_is_exhaustive_consistent_and_truthful() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("registry.db");
    setup_admin(&db);
    let config = ApiConfig::default();
    let token = login_token(&config, &db);

    let mut fetch = request("GET", "/v1/admin/commands", &config.frontend_origin);
    with_session(&mut fetch, &token);
    let response = handle(&config, &db, &fetch, Utc::now());
    assert_eq!(response.status, 200);
    assert_eq!(
        response
            .headers
            .get("content-type")
            .map(String::as_str)
            .unwrap_or(""),
        "application/json"
    );
    let body = body_json(&response);
    assert_eq!(body["contract"], CATALOG_CONTRACT);

    let commands = body["commands"].as_array().unwrap();
    let categories = body["categories"].as_array().unwrap();
    assert!(
        commands.len() >= 225,
        "catalog must cover every Clap path; got {}",
        commands.len()
    );

    let mut ids = BTreeSet::new();
    let id_set: BTreeSet<String> = commands
        .iter()
        .map(|row| row["id"].as_str().unwrap().to_string())
        .collect();
    let known_categories: BTreeSet<&str> = categories
        .iter()
        .map(|category| category["id"].as_str().unwrap())
        .collect();
    for row in commands {
        let id = row["id"].as_str().unwrap();
        assert!(ids.insert(id), "duplicate catalog id `{id}`");
        assert!(!row["summary"].as_str().unwrap().is_empty(), "{id}");

        match row["parent_id"].as_str() {
            Some(parent) => {
                assert!(id_set.contains(parent), "{id} references {parent}");
                assert!(id.starts_with(&format!("{parent}.")), "{id} vs {parent}");
            }
            None => assert!(!id.contains('.'), "top-level {id} is dotted"),
        }

        let availability = row["availability"].as_str().unwrap();
        match availability {
            "web" => {
                // A web row must resolve to one of the typed routes the API
                // actually implements: the fleet read, the workbench detail /
                // plan endpoints, the authoring `feature add` / `spec generate`
                // admin routes, the `forge-web-project-actions` lifecycle
                // writes (`feature remove` / `feature upgrade` /
                // `spec apply`), or the delivery share pipeline. No web row
                // may name a shell.
                let route = row["route"].as_str().unwrap_or("");
                assert!(
                    [
                        "GET /v1/admin/projects",
                        "GET /v1/admin/projects/{id}",
                        "GET /v1/admin/projects/{id}/plan",
                        "POST /v1/admin/projects/{id}/feature",
                        "POST /v1/admin/projects/{id}/feature/remove",
                        "POST /v1/admin/projects/{id}/feature/upgrade",
                        "POST /v1/admin/projects/{id}/spec",
                        "POST /v1/admin/projects/{id}/spec/apply",
                        "POST /v1/admin/projects/{id}/classify/approve",
                        "POST /v1/admin/projects/{id}/classify/reject",
                        "POST /v1/admin/projects/{id}/classify/apply",
                        "GET /v1/admin/projects/{id}/deploy/plan",
                        "POST /v1/admin/projects/{id}/deploy",
                        "GET /v1/admin/projects/{id}/release/plan",
                        "POST /v1/admin/projects/{id}/release",
                        "POST /v1/admin/projects/{id}/publish",
                        "GET /v1/admin/projects/{id}/delivery/status",
                        "POST /v1/admin/projects/{id}/delivery/preflight",
                        "POST /v1/admin/projects/{id}/delivery/stage",
                        "POST /v1/admin/projects/{id}/delivery/promote",
                        "POST /v1/admin/projects/{id}/delivery/hermora-retry",
                        "GET /v1/admin/projects/{id}/status",
                        "GET /v1/admin/status",
                        "POST /v1/admin/projects/new",
                        "POST /v1/admin/projects/import",
                        "POST /v1/admin/projects/register",
                        "GET /v1/admin/delivery",
                        "GET /v1/admin/delivery/preview",
                        "POST /v1/admin/delivery/allowlist/{id}",
                        "POST /v1/admin/delivery/allowlist/{id}/remove",
                        "POST /v1/admin/delivery/approve",
                        "POST /v1/admin/delivery/publish",
                        "POST /v1/admin/delivery/reconcile",
                    ]
                    .contains(&route),
                    "web row {id} points at unexpected route `{route}`"
                );
            }
            "cli_only"
            | "provider_required"
            | "project_capability_required"
            | "disabled"
            | "not_yet_web" => {
                let reason = row["reason"].as_str().unwrap_or("");
                assert!(!reason.is_empty(), "non-web row {id} lacks a reason");
                assert!(
                    row.get("route").is_none() || row["route"].is_null(),
                    "non-web row {id} must not name a route"
                );
            }
            other => panic!("row {id} has unknown availability `{other}`"),
        }
        assert!(
            ["read", "local_write", "remote_write", "session_admin"]
                .contains(&row["risk"].as_str().unwrap()),
            "{id} risk",
        );
        assert!(
            ["workspace", "project", "forge", "profile", "provider"]
                .contains(&row["scope"].as_str().unwrap()),
            "{id} scope",
        );
        assert!(
            known_categories.contains(row["category"].as_str().unwrap()),
            "{id} category",
        );
        assert_eq!(
            row["cli_invocation"].as_str().unwrap(),
            format!("forge {}", id.replace('.', " ")),
            "{id} invocation text",
        );
    }

    // Every named top-level command — transports and terminal commands
    // included — is discoverable in the catalog.
    for name in TOP_LEVEL_COMMANDS {
        assert!(ids.contains(name), "top-level `{name}` missing");
    }
    // Named nested spot-checks keep the coverage honest.
    for path in [
        "project.github.pull-request",
        "portfolio.share.reconcile",
        "delivery.hermora-retry",
        "identity.session-terminate",
        "publish.provider.enable",
        "agent.new-session",
        "ui-pattern.install",
        "inventory.show",
    ] {
        assert!(ids.contains(path), "nested `{path}` missing");
    }

    // Web availability is evidence-based: only routes that really exist. The
    // workbench (`forge-web-project-workbench`) turns inspect, doctor and
    // upgrade into typed single-project workflows; the delivery package
    // (`forge-web-delivery-controls`) turns the portfolio share pipeline
    // (allowlist, preview, approve, publish, reconcile, status) into typed
    // confirm- and digest-bound routes; and the command-execution and
    // project-actions packages (`forge-web-command-execution`,
    // `forge-web-project-actions`) turn the handler-backed authoring commands
    // (`feature add`/`spec generate`) and the lifecycle write commands
    // (`feature remove`/`feature upgrade`/`spec apply`) into typed, session-gated,
    // confirm/digest-bound browser routes. All now join the fleet read rows as
    // `web`.
    let web_ids: BTreeSet<&str> = commands
        .iter()
        .filter(|row| row["availability"] == "web")
        .map(|row| row["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        web_ids,
        BTreeSet::from([
            "list",
            "inspect",
            "register",
            "import",
            "new",
            "doctor",
            "check",
            "upgrade",
            "feature.add",
            "feature.remove",
            "feature.upgrade",
            "spec.generate",
            "spec.apply",
            "classify.apply",
            "classify.approve",
            "classify.reject",
            "release.prepare",
            "release.apply",
            "deploy.plan",
            "deploy.apply",
            "publish",
            "delivery.status",
            "delivery.preflight",
            "delivery.stage",
            "delivery.promote",
            "delivery.hermora-retry",
            "fleet.list",
            "fleet.status",
            "inventory.show",
            "portfolio.share.set",
            "portfolio.share.remove",
            "portfolio.share.show",
            "portfolio.share.list",
            "portfolio.share.preview",
            "portfolio.share.approve",
            "portfolio.share.publish",
            "portfolio.share.reconcile",
            "portfolio.share.audit",
        ])
    );
}

#[test]
fn post_to_the_catalog_is_method_not_allowed() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("registry.db");
    setup_admin(&db);
    let config = ApiConfig::default();
    let token = login_token(&config, &db);

    let mut post = request("POST", "/v1/admin/commands", &config.frontend_origin);
    with_session(&mut post, &token);
    post.body = json_body(&serde_json::json!({ "command": "list" }));
    let response = handle(&config, &db, &post, Utc::now());
    assert_eq!(response.status, 405);
}

#[test]
fn no_shell_or_eval_invocation_route_exists() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("registry.db");
    setup_admin(&db);
    let config = ApiConfig::default();
    let token = login_token(&config, &db);

    // Every plausible arbitrary-execution path must 404: the catalog is
    // descriptive and the API invokes nothing.
    for path in [
        "/v1/admin/exec",
        "/v1/admin/shell",
        "/v1/admin/run",
        "/v1/admin/invoke",
        "/v1/admin/commands/run",
        "/v1/admin/commands/execute",
        "/v1/commands/publish",
    ] {
        let mut fetch = request("GET", path, &config.frontend_origin);
        with_session(&mut fetch, &token);
        let response = handle(&config, &db, &fetch, Utc::now());
        assert_eq!(response.status, 404, "path {path} must not exist");
    }
}

#[test]
fn shell_metacharacters_in_requests_stay_literal_text() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("registry.db");
    setup_admin(&db);
    let config = ApiConfig::default();
    let token = login_token(&config, &db);

    let mut fetch = request("GET", "/v1/admin/commands", &config.frontend_origin);
    with_session(&mut fetch, &token);
    // A hostile query string must not echo, alter or execute anything:
    // the catalog body is static descriptive metadata.
    fetch.query = Some("search=;rm%20-rf%20%2F%20%24(ID)&command=publish".to_string());
    let response = handle(&config, &db, &fetch, Utc::now());
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    assert_eq!(body["contract"], CATALOG_CONTRACT);
    let rendered = response.body;
    let text = String::from_utf8_lossy(&rendered);
    assert!(
        !text.contains("rm -rf"),
        "the request string must never be echoed back"
    );
}

/// Runs the real binary's `--help` (top level plus two nested groups) and
/// asserts the catalog covers every named command the CLI actually renders.
#[test]
fn catalog_covers_the_real_help_output_of_the_binary() {
    let rendered = forge::api::command_catalog::rows();
    let ids: BTreeSet<&str> = rendered.iter().map(|row| row.id.as_str()).collect();

    let help_output = |args: &[&str]| -> String {
        let output = Command::new(env!("CARGO_BIN_EXE_forge"))
            .args(args)
            .arg("--help")
            .output()
            .expect("forge binary runs");
        assert!(
            output.status.success(),
            "`forge {} --help` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("help output is UTF-8")
    };

    let names_from_help = |text: &str| -> Vec<String> {
        // Clap renders `Commands:` at column zero, each entry indented by
        // exactly two spaces (wrapped descriptions indent by four) and the
        // section ends at the first column-zero `Options:`/`Arguments:`.
        let mut names = Vec::new();
        let mut in_commands = false;
        for line in text.lines() {
            if line.trim_end() == "Commands:" {
                in_commands = true;
                continue;
            }
            if in_commands
                && (line.starts_with("Options:")
                    || line.starts_with("Arguments:")
                    || (!line.starts_with(' ') && !line.is_empty()))
            {
                break;
            }
            if in_commands && line.starts_with("  ") && !line.starts_with("   ") {
                if let Some(name) = line.split_whitespace().next() {
                    names.push(name.to_string());
                }
            }
        }
        names
    };

    for name in names_from_help(&help_output(&[])) {
        assert!(
            ids.contains(name.as_str()),
            "`forge --help` lists `{name}` but the catalog does not cover it"
        );
    }
    // Nested groups render clap's generated `help` subcommand too; the
    // catalog represents terminal help once, as the top-level `help` row,
    // so nested `help` names are skipped here.
    for (group, prefix) in [("project", "project."), ("portfolio", "portfolio.")] {
        for name in names_from_help(&help_output(&[group])) {
            if name == "help" {
                continue;
            }
            let path = format!("{prefix}{name}");
            assert!(
                ids.contains(path.as_str()),
                "`forge {group} --help` lists `{name}` but the catalog does not cover `{path}`"
            );
        }
    }
}

#[test]
fn workbench_collapses_actions_into_choices_with_human_titles_and_field_hints() {
    let app = std::fs::read_to_string("frontend/app.js").unwrap();
    let css = std::fs::read_to_string("frontend/styles.css").unwrap();

    // The wall of pre-expanded forms was the reason the workbench read as
    // unhuman: every action rendered its inputs on screen at once, so a
    // project with a dozen executable rows showed a dozen forms before the
    // operator had chosen anything. Actions must be collapsed rows, and
    // opening one must close the others.
    assert!(
        app.contains("wb-action-head"),
        "each action must render a collapsed header row"
    );
    assert!(
        app.contains("body.hidden = !opening") && app.contains("aria-expanded"),
        "the header must toggle the form and report its expanded state"
    );
    assert!(
        app.contains("other.querySelector(\".wb-action-body\").hidden = true"),
        "opening one action must close every other action"
    );
    assert!(
        css.contains(".wb-action-head") && css.contains(".wb-action-caret"),
        "the collapsed row and its affordance need styles"
    );

    // `command.label` is only the leaf subcommand, so `spec apply`,
    // `release apply` and `deploy apply` all read as the single word "apply".
    // The title must come from the row's own human summary instead.
    assert!(
        app.contains("humanActionTitle"),
        "the action title must be derived, not the bare subcommand label"
    );
    assert!(
        app.contains("wb-action-cli"),
        "the CLI invocation belongs beside the title as secondary text, never as it"
    );

    // A field labelled only by its parameter name gives an operator nothing to
    // act on; every field needs a human label and, where a format is knowable,
    // an example.
    assert!(
        app.contains("paramLabel") && app.contains("paramHint"),
        "every action field needs a human label and a hint"
    );
    assert!(
        app.contains("Feature name") && app.contains("e.g. auth"),
        "the fields must say what to type, in the operator's words"
    );
    assert!(
        !app.contains("field.placeholder = param.kind === \"string_array\""),
        "a placeholder that echoes the parameter name is exactly the defect"
    );

    // The lifecycle card must read the flat manifest the API actually returns.
    // `GET /v1/admin/projects/{id}` returns `manifest` un-nested, so reading
    // `manifest.project.maturity` silently renders every field as "—".
    assert!(
        app.contains("(manifest && manifest.maturity)"),
        "the lifecycle card must read the flat manifest shape"
    );
    assert!(
        !app.contains("manifest.project.maturity"),
        "reading a nested path the API never returns leaves the card blank"
    );
    assert!(
        app.contains("nextStep"),
        "the lifecycle summary must distinguish no-target, at-target and working-toward"
    );
}

#[test]
fn frontend_uses_the_catalog_for_per_project_buttons_not_a_reference_page() {
    let app = std::fs::read_to_string("frontend/app.js").unwrap();
    let html = std::fs::read_to_string("frontend/index.html").unwrap();

    assert!(
        app.contains("/v1/admin/commands"),
        "app.js must fetch the catalog"
    );
    assert!(app.contains("credentials: \"include\""));
    assert!(
        app.contains("lifecycleStageFor"),
        "app.js must group the executable rows by lifecycle stage so the workbench renders one button per stage"
    );
    assert!(
        !app.contains("eval("),
        "the frontend must never evaluate command text"
    );

    // The catalog-as-reference page is gone. The workbench derives its
    // buttons from the same JSON endpoint; the operator never sees a
    // table of every CLI command.
    assert!(
        !html.contains("id=\"commands\""),
        "the standalone Command catalog page is removed; per-project buttons are the only surface"
    );
    assert!(
        !html.contains("id=\"command-search\""),
        "the catalog search input is removed"
    );
    assert!(
        !html.contains("id=\"command-rows\""),
        "the catalog table body is removed"
    );
    assert!(
        !html.contains("id=\"command-no-results\""),
        "the catalog empty state is removed"
    );
    // The workbench surfaces the catalog rows as buttons, in a grouped
    // card whose id is wb-actions.
    assert!(
        html.contains("id=\"wb-actions\"") && html.contains("wb-actions-grouped"),
        "the workbench renders the catalog rows as grouped action buttons"
    );

    // The catalog stays in Rust; nothing moved into the assets.
    let admin_rs = std::fs::read_to_string("src/api/admin.rs").unwrap();
    let catalog_rs = std::fs::read_to_string("src/api/command_catalog.rs").unwrap();
    for source in [&admin_rs, &catalog_rs] {
        assert!(
            !source.contains("<html") && !source.contains("<body") && !source.contains("html!"),
            "the catalog source must not embed HTML"
        );
    }
}
