//! SPA control-plane portal responsive + reduced-motion contract
//! (`forge-portal-responsive-a11y`).
//!
//! The change is pure CSS on `frontend/styles.css` (plus a
//! `:nth-child`-based label pattern that needs no `data-label` attribute
//! and therefore no `frontend/app.js` / `frontend/index.html` edit). This
//! light contract test pins the shipped static assets so the two required
//! media blocks exist and the eight onboarding-table column labels are
//! present, and that the existing a11y invariants in `index.html` survive.
//! It deliberately uses only `std` so it stays out of the `forge` crate
//! compile graph and can run without a backend.

use std::fs;

fn styles_css() -> String {
    fs::read_to_string("frontend/styles.css").expect("frontend/styles.css ships")
}

fn index_html() -> String {
    fs::read_to_string("frontend/index.html").expect("frontend/index.html ships")
}

#[test]
fn styles_carry_reduced_motion_and_720_breakpoint() {
    let css = styles_css();
    assert!(
        css.contains("@media (prefers-reduced-motion: reduce)"),
        "styles.css must carry a prefers-reduced-motion block"
    );
    assert!(
        css.contains("@media (max-width:720px)"),
        "styles.css must carry a 720px breakpoint"
    );
}

#[test]
fn reduced_motion_block_covers_rail_panels_cards() {
    let css = styles_css();
    // The reduced-motion guard must name the rail, a panel and a card
    // surface and must disable motion without hiding anything.
    let needs = [
        ".lifecycle-rail",
        ".project-panel",
        ".wb-card",
        "transition:none!important",
    ];
    for token in needs {
        assert!(
            css.contains(token),
            "reduced-motion block must reference `{token}`"
        );
    }
    assert!(
        !css.contains("prefers-reduced-motion: reduce){}"),
        "reduced-motion block must not be an empty rule"
    );
}

#[test]
fn onboarding_table_reflow_labels_all_eight_columns() {
    let css = styles_css();
    // The 720px block must reflow #ws-table-wrap into stacked cards and
    // label every one of the eight columns from column position.
    assert!(
        css.contains("#ws-table-wrap"),
        "720px block must target the workspace onboarding table wrapper"
    );
    let labels = [
        "Onboard",
        "Directory",
        "Project id",
        "Action",
        "Profile signal",
        "State",
        "Id override",
        "Profile override",
    ];
    for label in labels {
        assert!(
            css.contains(label),
            "720px block must label the `{label}` column"
        );
    }
    // Count the eight :nth-child label rules as a proxy that no column was
    // dropped from the reflow.
    let nth_count = css.matches("::before{content:\"").count();
    assert!(
        nth_count >= 8,
        "expected >=8 column-label ::before rules, found {nth_count}"
    );
}

#[test]
fn index_html_keeps_a11y_invariants() {
    let html = index_html();
    // Skip link must remain.
    assert!(
        html.contains("class=\"skip-link\""),
        "skip-link must remain in index.html"
    );
    // At least the workspace status live region must remain.
    assert!(
        html.contains("role=\"status\"") && html.contains("aria-live"),
        "role=status / aria-live regions must remain in index.html"
    );
    // The onboarding table region landmark must remain intact.
    assert!(
        html.contains("aria-label=\"Workspace candidates\""),
        "workspace onboarding table region landmark must remain"
    );
}
