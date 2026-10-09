//! Contract test for `portal-feedback-session-recovery`.
//!
//! Static assertions over the shipped `frontend/` assets:
//!
//! - every `.wb-plan-result` container is a live region (`role="status"`);
//! - `app.js` sets the result role per outcome (polite success, assertive
//!   refusal) through one shared helper;
//! - `request`/`requestStatus` return the operator to sign-in on a mid-session
//!   `401`, carrying the current deep link, and never redirect from the login
//!   page;
//! - the slice-7 stylesheet raises essential text to the 12px floor;
//! - workbench disclosure buttons carry `aria-controls` for their panel;
//! - the prior slices' tokens remain intact.

use std::path::Path;

fn read(name: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(name))
        .unwrap_or_else(|err| panic!("read {name}: {err}"))
}

#[test]
fn result_containers_are_live_regions() {
    let html = read("frontend/index.html");
    let app = read("frontend/app.js");

    // Every static result container declares a polite live region in markup.
    let containers = html.matches("class=\"wb-plan-result\"").count();
    assert_eq!(
        containers, 7,
        "expected seven .wb-plan-result containers in index.html"
    );
    let with_role = html
        .matches("class=\"wb-plan-result\" role=\"status\"")
        .count();
    assert_eq!(
        with_role, containers,
        "every .wb-plan-result container must declare role=\"status\""
    );

    // One shared helper sets role + aria-live per outcome.
    for token in [
        "function setResultRole",
        "node.setAttribute(\"role\", isError ? \"alert\" : \"status\")",
        "node.setAttribute(\"aria-live\", isError ? \"assertive\" : \"polite\")",
    ] {
        assert!(app.contains(token), "app.js must implement `{token}`");
    }

    // The helper is wired into every result writer, and refusals announce
    // assertively.
    assert!(
        app.matches("setResultRole(").count() >= 18,
        "setResultRole must be wired into every result writer"
    );
    let assertive = app.matches("setResultRole(box, true)").count()
        + app.matches("setResultRole(result, true)").count();
    assert!(
        assertive >= 4,
        "each refusal path (workbench, delivery, workspace, scoped management, action card) must announce assertively"
    );
    assert!(
        app.contains("setResultRole(box, Boolean(result.error))"),
        "a delivery result must announce assertively when it carries an error"
    );
}

#[test]
fn mid_session_401_returns_to_sign_in() {
    let app = read("frontend/app.js");

    for token in [
        "function sessionExpiredRedirect",
        "if (page === \"login\") return false;",
        "window.location.replace(`login.html?next=${encodeURIComponent(here)}`)",
        "response.status === 401 && sessionExpiredRedirect()",
        "if (response.status === 401) sessionExpiredRedirect()",
        "sessionExpired: true",
    ] {
        assert!(app.contains(token), "app.js must implement `{token}`");
    }

    // Both request helpers handle 401; the login page keeps its own path.
    let request_fn = app
        .split_once("const request = async")
        .and_then(|(_, rest)| rest.split_once("const requestStatus = async"))
        .map(|(request, _)| request)
        .expect("request helper");
    assert!(
        request_fn.contains("401"),
        "the primary request helper must handle 401"
    );
    let status_fn = app
        .split_once("const requestStatus = async")
        .and_then(|(_, rest)| rest.split_once("const session = ()"))
        .map(|(status, _)| status)
        .expect("requestStatus helper");
    assert!(
        status_fn.contains("401"),
        "the status request helper must handle 401"
    );

    // The login submit keeps rendering its inline error: it must not redirect
    // itself, and the redirect guard names the login page.
    assert!(
        app.contains("window.location.assign(next || \"index.html\")"),
        "successful login must still navigate to the validated next target"
    );
}

#[test]
fn essential_text_meets_the_twelve_px_floor() {
    let css = read("frontend/styles.css");

    assert!(
        css.contains("--text-md:12px"),
        "the type scale must define the 12px mid step"
    );
    for selector in [
        ".source-reason",
        ".source-malformed",
        ".source-seen",
        ".source-meta",
        ".wb-digest",
        ".wb-action-cli",
        ".detail-row",
        ".wb-findings",
        ".workflow-reason",
        ".field-hint",
        ".field-error",
        ".fleet-filter-error",
        ".updated-label",
        ".evidence-chip",
        ".summary-card p",
        ".app-footer",
    ] {
        assert!(
            css.contains(selector),
            "slice-7 must floor `{selector}` at the 12px step"
        );
    }
    assert!(
        css.contains("slice 7: essential text floor"),
        "the floor must live in one labelled override block"
    );
    // Base size and touch minima from prior slices survive.
    assert!(css.contains("html{font-size:16px}"), "base size unchanged");
    assert!(css.contains("min-height:44px"), "touch minima unchanged");
}

#[test]
fn disclosure_buttons_control_their_panel() {
    let app = read("frontend/app.js");

    for token in [
        "let actionBodySeq = 0;",
        "body.id = `wb-action-body-${actionBodySeq++}`",
        "head.setAttribute(\"aria-controls\", body.id)",
        "head.setAttribute(\"aria-expanded\", \"false\")",
    ] {
        assert!(app.contains(token), "app.js must implement `{token}`");
    }
}

#[test]
fn prior_slice_tokens_survive() {
    let html = read("frontend/index.html");
    let css = read("frontend/styles.css");
    let app = read("frontend/app.js");

    for token in [
        "min-height:44px",
        "touch-action:manipulation",
        "renderErrorSummary",
        "fleet-filter-error",
        "--focus:#9aa5ff",
        "forge.filter-state.v1",
        "view-unknown",
        "--z-skip:60",
        "applyWorkbenchProjectParam",
        "loginNextTarget",
        "scrollIntoViewRespectingMotion",
        "--icon-stroke:1.8",
        "--dur-enter:140ms",
        "prefers-reduced-motion",
        "aria-sort",
        "Export filtered CSV",
        "FLEET_PAGE_SIZE",
        "renderFleetSkeleton",
    ] {
        let present = html.contains(token) || css.contains(token) || app.contains(token);
        assert!(present, "prior-slice token `{token}` must stay intact");
    }
}
