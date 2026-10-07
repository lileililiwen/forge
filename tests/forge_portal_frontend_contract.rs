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
    // Verb summaries read as plain words; operation ids never render.
    assert!(
        app.contains("deliveryVerbSummary"),
        "verb summaries are rendered as sentences"
    );
    assert!(
        !app.contains("confirm_operation_id:") && !app.contains("confirm_revision:"),
        "no staged confirmation codes rendered as text"
    );
    // System operation ids never render; the only remaining `operation ${`
    // echoes are the operator's own typed operation key, never a hash.
    for marker in [
        "operation ${verb",
        "operation ${result",
        "operation ${hermora",
        "operation ${body",
        "operation ${applied",
    ] {
        assert!(
            !app.contains(marker),
            "system operation id rendered: {marker}"
        );
    }
    // Staged confirmations arrive pre-filled from the delivery status the
    // operator reviewed instead of hand-copied hashes.
    assert!(
        app.contains("workbench.delivery") && app.contains("prefill"),
        "staged confirmations are pre-filled from status data"
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
fn fleet_rows_lead_with_human_names_and_one_plain_status() {
    let index = read("frontend/index.html");
    let app = read("frontend/app.js");
    assert!(
        index.contains("<th scope=\"col\">Project</th><th scope=\"col\">Details</th><th scope=\"col\">Status</th>"),
        "fleet headers are Project/Details/Status"
    );
    // The old programmer-coded fleet header row is gone from the primary
    // view (portfolio/delivery reference tables keep their own columns).
    assert!(
        !index.contains("<th scope=\"col\">Source</th><th scope=\"col\">Profile</th>"),
        "no source/lifecycle/access/evidence code columns in the primary fleet view"
    );
    assert!(
        app.contains("projectStatusLine") && app.contains("profileWords"),
        "one plain status line plus profile words"
    );
    assert!(
        app.contains("\"react-web\": \"React web\""),
        "all six known profiles map to plain words"
    );
    assert!(
        app.contains("Open ${project.name}") || app.contains("\"Open\""),
        "managed rows offer a plain Open action"
    );
}

#[test]
fn primary_views_render_no_hashes_digests_or_op_ids() {
    let app = read("frontend/app.js");
    for marker in [
        "Plan digest:",
        "Action digest:",
        "Batch 1 digest",
        "Refreshed preview digest:",
        "Accepted as journaled operation",
    ] {
        assert!(!app.contains(marker), "display marker removed: {marker}");
    }
    // Digests stay in JS memory and wire bodies only; the screen shows
    // 12-char prefixes at most.
    assert!(
        app.contains("shortValue") && app.contains("shortDigest"),
        "hex values are truncated before display"
    );
    assert!(
        app.contains("slice(0, 12)"),
        "12-char prefix bound for operationally needed references"
    );
    assert!(
        app.contains("(\"Approved reference\", shortDigest(approval.manifest_sha256))"),
        "approval digest renders as a short reference"
    );
}

#[test]
fn onboarding_refreshes_the_fleet_in_place_with_manual_fallback() {
    let index = read("frontend/index.html");
    let app = read("frontend/app.js");
    assert!(
        app.contains("wsReloadFleet"),
        "fleet re-fetch helper exists"
    );
    assert!(
        app.contains("wsReloadFleet()") && app.contains("/v1/admin/projects"),
        "onboard success re-reads the fleet in place"
    );
    assert!(
        index.contains("id=\"ws-reload\""),
        "manual Reload fallback is retained"
    );
    assert!(
        app.contains("could not refresh itself"),
        "failed auto-refresh names the manual fallback"
    );
}

#[test]
fn stylesheet_pins_the_dark_command_center_tokens() {
    let css = read("frontend/styles.css");
    for token in [
        "--bg:#08090d",
        "--panel:#0f1219",
        "--accent:#5e6ad2",
        "color-scheme:dark",
    ] {
        assert!(css.contains(token), "dark token pinned: {token}");
    }
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
