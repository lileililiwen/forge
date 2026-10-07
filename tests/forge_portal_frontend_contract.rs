//! Source-level contract for the standalone Forge portal frontend.
//!
//! The browser pages are owned by `frontend/` and served by the Rust static
//! web listener; the API returns JSON only. These assertions pin the split
//! that `forge-global-admin-portal` requires: the login has no project-id
//! control and no fake actions, the dashboard drives the authenticated JSON
//! API, the stylesheet keeps the accessibility contract, and no page markup
//! is embedded in Rust.

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn read(relative: &str) -> String {
    fs::read_to_string(repo_root().join(relative))
        .unwrap_or_else(|err| panic!("missing {relative}: {err}"))
}

#[test]
fn all_browser_assets_live_under_frontend() {
    for name in [
        "frontend/login.html",
        "frontend/index.html",
        "frontend/styles.css",
        "frontend/config.js",
        "frontend/app.js",
        "frontend/README.md",
    ] {
        assert!(
            repo_root().join(name).is_file(),
            "expected standalone asset {name}"
        );
    }
}

#[test]
fn login_page_has_only_email_and_password_and_no_project_id() {
    let login = read("frontend/login.html");
    assert!(login.contains("type=\"email\""), "email control required");
    assert!(
        login.contains("type=\"password\""),
        "password control required"
    );
    assert!(!login.contains("name=\"project\""), "no project-id control");
    assert!(
        !login.contains("name=\"project_id\""),
        "no project-id control"
    );
    // Exactly the two credential inputs are present.
    assert_eq!(
        login.matches("<input").count(),
        2,
        "login must offer only email and password fields"
    );
    // Credentials never travel in the URL; the form posts via app.js.
    assert!(!login.contains("action=\"http"), "no credential URL action");
}

#[test]
fn dashboard_drives_the_json_api_with_credentials_and_honest_states() {
    let index = read("frontend/index.html");
    let app = read("frontend/app.js");
    assert!(index.contains("type=\"search\""), "project search control");
    assert!(index.contains("<table"), "project roster table");
    assert!(index.contains("id=\"sign-out\""), "sign-out control");
    assert!(
        app.contains("credentials: \"include\""),
        "credentialed fetch"
    );
    assert!(app.contains("/v1/admin/session"), "session endpoint");
    assert!(app.contains("/v1/admin/projects"), "fleet endpoint");
    assert!(app.contains("empty-state"), "honest empty fleet state");
    assert!(
        app.contains("Forge API is unavailable"),
        "honest unavailable state"
    );
    // No fabricated metrics or fake project actions: the summary values come
    // from the authenticated API, and no password/token is persisted.
    assert!(!app.contains("localStorage"), "no credential/token storage");
    assert!(
        !app.contains("sessionStorage"),
        "no credential/token storage"
    );
}

#[test]
fn workbench_has_a_delivery_status_card_with_next_confirmation() {
    let index = read("frontend/index.html");
    let app = read("frontend/app.js");
    assert!(index.contains("id=\"wb-delivery\""), "delivery card");
    assert!(
        index.contains("id=\"wb-delivery-title\""),
        "delivery card label"
    );
    assert!(
        index.contains("role=\"status\"") && index.contains("aria-live=\"polite\""),
        "delivery live region"
    );
    assert!(app.contains("/delivery/status"), "delivery status endpoint");
    assert!(
        app.contains("confirm_operation_id:") && app.contains("confirm_revision:"),
        "next staged confirmations are shown as text"
    );
    assert!(
        !app.contains("deployment_url: ${"),
        "Hermora inputs stay inputs rather than rendered values"
    );
}

#[test]
fn dashboard_orders_work_before_reference_with_live_onboarding() {
    let index = read("frontend/index.html");
    let app = read("frontend/app.js");
    let pos = |id: &str| index.find(&format!("id=\"{id}\"")).unwrap();
    assert!(
        pos("workbench-title") < pos("management-title")
            && pos("management-title") < pos("commands-title"),
        "dashboard order must be workbench < management < commands"
    );
    assert!(
        index.contains("id=\"ws-fleet-hint\""),
        "unonboarded count line"
    );
    assert!(
        app.contains("wsDiscover();") && app.contains("not yet onboarded"),
        "discovery auto-runs and reports the unonboarded count"
    );
    assert!(
        app.contains("WS_CHUNK = 25"),
        "chunked one-confirm onboarding bound to the server batch cap"
    );
}

#[test]
fn management_has_a_workspace_onboarding_panel() {
    let index = read("frontend/index.html");
    let app = read("frontend/app.js");
    assert!(index.contains("id=\"ws-discover\""), "discover control");
    assert!(index.contains("id=\"ws-rows\""), "candidate table");
    assert!(index.contains("id=\"ws-preview\""), "preview control");
    assert!(index.contains("id=\"ws-run\""), "confirmed run control");
    assert!(index.contains("role=\"status\""), "live status region");
    assert!(
        app.contains("/v1/admin/workspace/candidates"),
        "discovery endpoint"
    );
    assert!(
        app.contains("/v1/admin/workspace/onboard"),
        "onboard endpoint"
    );
    assert!(
        app.contains("wsSelected") && app.contains("plan_digest"),
        "selection is previewed under a digest before any write"
    );
    // No browser-supplied path can reach the API: the panel sends directory
    // leaves and typed overrides only.
    assert!(
        !app.contains("wsRoot") && !app.contains("rootPath") && !app.contains("directoryPath"),
        "no root/path plumbing in the panel"
    );
}

#[test]
fn stylesheet_keeps_the_accessibility_and_responsive_contract() {
    let css = read("frontend/styles.css");
    assert!(css.contains("min-width:320px"), "320px readability floor");
    assert!(
        css.contains("@media(max-width:560px)") || css.contains("@media (max-width: 560px)"),
        "narrow reflow breakpoint"
    );
    assert!(css.contains(":focus-visible"), "visible focus indicator");
    assert!(css.contains("skip-link"), "skip-to-content affordance");
    assert!(
        css.contains("prefers-reduced-motion"),
        "reduced-motion support"
    );
}

#[test]
fn rust_sources_embed_no_browser_markup() {
    // The change must keep HTML/CSS/JS out of the Rust API and web server.
    for source in ["src/api/admin.rs", "src/web.rs", "src/identity/global.rs"] {
        let text = read(source);
        let lower = text.to_ascii_lowercase();
        assert!(
            !lower.contains("<html") && !lower.contains("<body") && !text.contains("html!"),
            "{source} must not render browser markup; frontend owns the pages"
        );
    }
    // The API admin handler is JSON-only.
    let admin = read("src/api/admin.rs");
    assert!(
        admin.contains("ApiResponse::json"),
        "admin API returns JSON"
    );
}
