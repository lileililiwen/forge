//! Contract tests for the read-only command-reference browser
//! (`web-command-reference-browser`, web UI/UX audit gap 1).
//!
//! Static-token contract over the shipped `frontend/` assets (no browser,
//! no server): the reference section lists every
//! `GET /v1/admin/commands` catalog row from the already-fetched catalog
//! JSON with a search + availability filter, an availability badge and
//! the exact CLI string on every row, the plain-language reason plus a
//! clipboard Copy button (and never an executable control) on every
//! non-web row, and a link to the existing serving view on every web
//! row. No new endpoint, no new route, no shell.

fn app_js() -> String {
    std::fs::read_to_string("frontend/app.js").expect("frontend/app.js ships")
}

fn index_html() -> String {
    std::fs::read_to_string("frontend/index.html").expect("frontend/index.html ships")
}

fn styles_css() -> String {
    std::fs::read_to_string("frontend/styles.css").expect("frontend/styles.css ships")
}

#[test]
fn reference_section_lives_on_the_projects_view() {
    let html = index_html();
    let app = app_js();

    for token in [
        "id=\"command-reference\"",
        "id=\"cmdref-title\"",
        "Command reference",
        "id=\"cmdref-search\"",
        "id=\"cmdref-availability\"",
        "id=\"cmdref-count\"",
        "id=\"cmdref-copy-status\"",
        "id=\"cmdref-list\"",
    ] {
        assert!(html.contains(token), "index.html must declare `{token}`");
    }
    // The section sits inside the projects view, not behind a new route:
    // no web-listener, router-table or sidebar change accompanies it.
    let projects = html
        .split_once("id=\"view-projects\"")
        .map(|(_, rest)| rest)
        .expect("projects view");
    let section_at = projects
        .find("id=\"command-reference\"")
        .expect("section in projects view");
    let workbench_at = projects
        .find("id=\"workbench\"")
        .expect("workbench follows");
    assert!(
        section_at < workbench_at,
        "the reference section must live inside the projects view"
    );
    for token in [
        "referenceViewForRoute",
        "renderCommandReference",
        "initCommandReference",
        "cmdrefRow",
        "cmdrefMatches",
    ] {
        assert!(app.contains(token), "app.js must implement `{token}`");
    }
    // Both catalog outcomes render the reference; the boot wires it once.
    assert!(
        app.contains("renderCommandReference();"),
        "both loadCommands arms must re-render the reference"
    );
    assert!(
        app.contains("initCommandReference();"),
        "the dashboard boot must wire the reference controls"
    );
}

#[test]
fn reference_filters_by_search_and_all_six_availability_states() {
    let html = index_html();
    let app = app_js();

    for value in [
        "value=\"web\"",
        "value=\"cli_only\"",
        "value=\"provider_required\"",
        "value=\"project_capability_required\"",
        "value=\"disabled\"",
        "value=\"not_yet_web\"",
    ] {
        assert!(
            html.contains(value),
            "the availability filter must offer `{value}`"
        );
    }
    for token in [
        "cmdref-search",
        "cmdref-availability",
        "cmdref-count",
        "role=\"status\"",
    ] {
        assert!(
            html.contains(token),
            "search, filter and the live count must be present in `{token}`"
        );
    }
    // The count and the copy feedback are live regions set on every write.
    assert!(
        app.contains("setResultRole(count, false)"),
        "the count must announce politely on every render"
    );
    assert!(
        app.contains("setResultRole(status, !copied)"),
        "copy feedback must announce success politely and failure assertively"
    );
    assert!(
        app.contains("Showing ${rows.length} of ${total} commands"),
        "the count must name the narrowed and total sets honestly"
    );
    // Empty states stay honest: unavailable catalog vs no filter matches.
    assert!(
        app.contains("Command reference unavailable"),
        "an unloadable catalog must render an unavailable state"
    );
    assert!(
        app.contains("No matching commands"),
        "a filter with no hits must say so"
    );
}

#[test]
fn every_row_shows_badge_summary_and_exact_cli_with_reason_and_copy_off_web() {
    let app = app_js();

    // Badge + category reuse the existing catalog vocabulary tables.
    for token in [
        "AVAILABILITY_LABELS[row.availability]",
        "AVAILABILITY_BADGE[row.availability]",
        "CATEGORY_LABELS[row.category]",
    ] {
        assert!(app.contains(token), "rows must reuse `{token}`");
    }
    // Reason rendering: the catalog reason when present, an honest fallback
    // when a non-web row carries none (a catalog-invariant violation).
    assert!(
        app.contains("cmdref-reason"),
        "non-web rows must render their plain-language reason"
    );
    assert!(
        app.contains("No reason was provided for this state"),
        "a missing reason must degrade to words, never to silence"
    );
    // The CLI string is the catalog's exact `cli_invocation`, rendered as
    // text inside `<code>`, with a clipboard Copy button beside it.
    assert!(
        app.contains("row.cli_invocation"),
        "rows must show the catalog's exact CLI string"
    );
    assert!(
        app.contains("cmdrefCopyButton"),
        "non-web rows must offer a clipboard Copy button"
    );
    assert!(
        app.contains("navigator.clipboard.writeText"),
        "copy must use the async clipboard API"
    );
    assert!(
        app.contains("Copy unavailable"),
        "a denied clipboard must degrade to a select-the-text note"
    );
    // Non-web rows never execute: no run/execute/apply control exists for
    // reference rows, and the block issues no fetch of its own.
    assert!(
        !app.contains("cmdrefRun")
            && !app.contains("cmdrefExecute")
            && !app.contains("cmdrefApply"),
        "no run/execute/apply control may exist for reference rows"
    );
}

#[test]
fn web_rows_link_to_their_existing_view_and_nothing_else() {
    let app = app_js();

    assert!(
        app.contains("Open in ${CMDREF_VIEW_LABELS[view]"),
        "web rows must link to their serving view in operator words"
    );
    for path in [
        "/projects",
        "/workbench",
        "/management",
        "/portfolio",
        "/delivery",
    ] {
        assert!(
            app.contains(&format!("\"{path}\"")),
            "the route mapping must be able to target `{path}`"
        );
    }
    // The mapping covers delivery, portfolio, workspace, the three
    // server-rooted management creations, the fleet reads, and the
    // project-scoped fallthrough — with a safe default, never a dead link.
    for token in [
        "referenceViewForRoute",
        "POST /v1/admin/projects/new",
        "POST /v1/admin/projects/import",
        "POST /v1/admin/projects/register",
        "GET /v1/admin/projects",
        "/v1/admin/status",
    ] {
        assert!(
            app.contains(token),
            "the route mapping must account for `{token}`"
        );
    }
    // Links are real-path anchors the router already serves (hard-load
    // safe); the reference adds no route of its own.
    assert!(
        !app.contains("/reference"),
        "the reference must not invent a new route"
    );
}

#[test]
fn reference_adds_no_endpoint_no_shell_and_no_html_interpretation() {
    let app = app_js();

    // The only catalog read stays the one `loadCommands` already makes.
    assert!(
        app.contains("request(\"/v1/admin/commands\""),
        "the catalog read must stay the existing one"
    );
    assert!(
        app.matches("request(\"/v1/admin/commands\"").count() == 1,
        "the reference must not add a second catalog fetch"
    );
    // No shell, no eval, no HTML interpretation of catalog strings: every
    // catalog field in the new block flows through the textContent-only
    // `el()` helper. (Two pre-existing `innerHTML` uses live elsewhere in
    // app.js under a separate pin; the reference block must add none, so
    // the block is bounded to the inserted section exactly.)
    let after = app.split("Command reference browser").nth(1).unwrap_or("");
    let block = after.split("---- Project workbench").next().unwrap_or("");
    assert!(
        !block.is_empty(),
        "the reference block must sit ahead of the workbench section"
    );
    for token in [
        "eval(",
        "new Function(",
        ".innerHTML",
        "child_process",
        "execSync",
        "shellQuote(",
    ] {
        assert!(
            !block.contains(token),
            "the reference block must not contain `{token}`"
        );
    }
    assert!(
        app.contains("createDocumentFragment"),
        "the 234-row render must batch into one fragment append"
    );
}

#[test]
fn reference_styles_meet_the_text_floor_with_existing_tokens() {
    let css = styles_css();

    for token in [
        ".cmdref-list",
        ".cmdref-row",
        ".cmdref-head",
        ".cmdref-summary",
        ".cmdref-reason",
        ".cmdref-cli",
        ".cmdref-open",
        ".cmdref-copy",
        ".cmdref-count",
    ] {
        assert!(css.contains(token), "styles.css must style `{token}`");
    }
    // Essential text at the 12px floor via the shared token; CLI wraps.
    assert!(
        css.contains("font-size:var(--text-md)"),
        "reference text must sit on the 12px floor token"
    );
    assert!(
        css.contains(".cmdref-cli code"),
        "the CLI line must have its own wrapping rule"
    );
    assert!(
        css.contains("overflow-wrap:anywhere"),
        "long CLI strings must wrap instead of overflowing"
    );
    // Touch targets reuse the 44px floor on the new controls.
    assert!(
        css.contains(".cmdref-field input"),
        "the search control must have a touch-target rule"
    );
}
