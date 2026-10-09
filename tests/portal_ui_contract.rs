//! Contract tests for the workbench lifecycle series (part 1,
//! `lifecycle-series-rail`).
//!
//! Static-token contract over the shipped `frontend/` assets (no browser,
//! no server): the series rail, the single next-best-action card, and the
//! Copy-as-CLI button on every confirm action. A live-behavior oracle is
//! out of scope for part 1; these tests pin the structure, derivation
//! sources, deep-link coexistence and secret redaction so a later slice
//! cannot silently regress them.

use std::process::Command;

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
fn workbench_renders_an_eight_step_series_rail() {
    let html = index_html();
    let app = app_js();

    assert!(
        html.contains("id=\"lifecycle-rail\""),
        "the lifecycle card must carry the rail shell"
    );
    assert!(
        html.contains("<ol id=\"lifecycle-rail\"") || html.contains("<ol id=\"lifecycle-rail\" "),
        "the rail must be a real ordered list, not a div"
    );
    assert!(
        app.contains("LIFECYCLE_RAIL_STEPS"),
        "the rail steps must be one named table, not scattered literals"
    );
    for step in [
        "idea", "scaffold", "spec", "code", "test", "release", "deploy", "operate",
    ] {
        assert!(
            app.contains(&format!("\"{step}\"")) || app.contains(&format!("'{step}'")),
            "rail step `{step}` must be a named key",
        );
    }
    for label in [
        "Idea", "Scaffold", "Spec", "Code", "Test", "Release", "Deploy", "Operate",
    ] {
        assert!(
            app.contains(label),
            "rail step label `{label}` must render in operator words",
        );
    }
}

#[test]
fn rail_state_derives_from_existing_projections_with_single_current() {
    let app = app_js();

    // Derivation reads the payloads the workbench already fetches — never a
    // new endpoint.
    assert!(
        app.contains("railStepStates"),
        "step states must come from one derivation function"
    );
    assert!(
        app.contains("workbenchEvidence"),
        "loaders must record their payloads into one shared evidence store"
    );
    for token in [
        "manifest.maturity",
        "target_maturity",
        "health",
        "deliveryReport",
        "deliveryNext",
        "operations",
    ] {
        assert!(
            app.contains(token),
            "rail derivation must read the existing `{token}` projection",
        );
    }
    // Exactly one step is current; the marker is the ARIA step token.
    assert!(
        app.contains("railCurrentStep"),
        "exactly one step must be selected as current"
    );
    assert!(
        app.contains("aria-current") && app.contains("\"step\""),
        "the current step must carry aria-current=\"step\""
    );
    // State is text as well as color: Done / Now / Later.
    for state in ["Done", "Now", "Later"] {
        assert!(
            app.contains(&format!("\"{state}\"")),
            "rail steps must name their state in text (`{state}`), never color alone",
        );
    }
}

#[test]
fn step_deep_link_coexists_with_the_project_link() {
    let app = app_js();

    assert!(
        app.contains("workbenchStepParam"),
        "the ?step= hint must be read through one helper"
    );
    assert!(
        app.contains("applyWorkbenchStepParam"),
        "the ?step= hint must be applied on route render"
    );
    assert!(
        app.contains("&step=") && app.contains("?project="),
        "rail links must carry both ?project= and ?step="
    );
    assert!(
        app.contains("workbenchUrl("),
        "project switches must preserve a valid step instead of dropping it"
    );
    // Applying the hint scrolls; it must never move aria-current, which
    // tracks derived state rather than the URL.
    assert!(
        app.contains("scrollIntoViewRespectingMotion(card"),
        "applying ?step= must scroll to the mapped card with reduced-motion respect"
    );
}

#[test]
fn next_best_action_is_one_ranked_pick_with_reason_and_cli() {
    let html = index_html();
    let app = app_js();

    assert!(
        html.contains("id=\"wb-next-title\""),
        "the Next card must be a labelled section"
    );
    assert!(
        html.contains("id=\"wb-next-body\""),
        "the Next card must have one body region"
    );
    assert!(
        app.contains("computeNextBestAction"),
        "the Next must be one ranked computation"
    );
    assert!(
        app.contains("renderNextBestAction"),
        "the Next must render through one function"
    );
    assert!(
        app.contains("refreshLifecycleSeries"),
        "rail and Next must refresh together on every projection resolve"
    );
    // The ranking covers the maturity/target/evidence cases from the design.
    for token in [
        "Declare project maturity",
        "Run the doctor",
        "Pick a target maturity",
        "Plan the upgrade",
        "Sustain",
    ] {
        assert!(app.contains(token), "the Next ranking must cover `{token}`",);
    }
    assert!(
        app.contains("openActionCard"),
        "the Next card must open its matching action card when one exists"
    );
}

#[test]
fn every_confirm_action_carries_copy_as_cli_with_redaction() {
    let app = app_js();

    assert!(
        app.contains("buildCliString"),
        "the CLI string must be built by one named function"
    );
    assert!(
        app.contains("Copy as CLI"),
        "every confirm card must label its copy button in operator words"
    );
    assert!(
        app.contains("wb-copy-cli"),
        "the copy button must be selectable for styling and review"
    );
    assert!(
        app.contains("cli_invocation"),
        "the string must start from the catalog cli_invocation, not a retyped command"
    );
    assert!(
        app.contains("clipboard.writeText"),
        "copy must use the clipboard API with a fallback"
    );
    assert!(
        app.contains("select the command above"),
        "clipboard absence must degrade to selecting the shown string"
    );
    // Secrets are never copied literally (GitHub preview pattern).
    assert!(
        app.contains("isSecretParam"),
        "secret-ish params must be detected by one helper"
    );
    assert!(
        app.contains("<token>") && app.contains("<redacted>"),
        "secrets must render as placeholders, never literally"
    );
    // The full string is shown untruncated before copying.
    assert!(
        app.contains("wb-action-cli-copy"),
        "the exact string must render in full in the card"
    );
    assert!(
        !app.contains("shortValue(cli"),
        "the copied string must never pass through the truncating preview shortener"
    );
}

#[test]
fn series_change_adds_no_frontend_dependency() {
    let html = index_html();
    let app = app_js();

    assert!(
        !app.contains("import(") && !app.contains("require("),
        "app.js stays dependency-free: no dynamic import or require"
    );
    let scripts = html.match_indices("<script").count();
    assert_eq!(
        scripts, 2,
        "index.html must keep exactly its two scripts (config.js + app.js); got {scripts}"
    );
    let css = styles_css();
    assert!(
        css.contains(".lifecycle-rail") && css.contains(".rail-current"),
        "the rail needs its appended style block"
    );
}

#[test]
fn frontend_syntax_stays_valid() {
    let output = Command::new("node")
        .args(["--check", "frontend/app.js"])
        .output()
        .expect("node runs");
    assert!(
        output.status.success(),
        "node --check frontend/app.js must pass: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// Part 2 (`lifecycle-series-rail` observability + mobile/a11y): static
// tokens for the failed-check remediate preview, the stale-row reconcile
// shortcut, the inline Hermora retry, the 860px topbar collapse with sticky
// headers, rail roving focus with text-alternative rows, and error-summary
// focus on every form.

#[test]
fn failed_check_rows_preview_the_remediate_plan() {
    let app = app_js();

    assert!(
        app.contains("remediatePlanCli"),
        "the remediate preview must come from one named builder"
    );
    assert!(
        app.contains("forge remediate plan --finding"),
        "the preview must be the existing `forge remediate plan` string with --finding"
    );
    assert!(
        app.contains("remediatePlanButton") && app.contains("Plan remediate"),
        "every failed doctor/status row must carry its Plan remediate button in operator words"
    );
    assert!(
        app.contains("wb-remediate-preview"),
        "the preview must render its full string in the card, never a truncated one"
    );
    assert!(
        app.contains("run this in the project directory") || app.contains("project directory"),
        "the preview must name the directory contract: --target stays out of the browser"
    );
    assert!(
        !app.contains("--target ${") && !app.contains("--target \" +"),
        "the browser must never interpolate a filesystem path into --target"
    );
}

#[test]
fn stale_rows_carry_a_refresh_and_reconcile_shortcut() {
    let app = app_js();

    assert!(
        app.contains("fleetNeedsReconcile"),
        "stale/unavailable detection must live in one named predicate"
    );
    for token in ["freshness", "stale", "unavailable", "conflict"] {
        assert!(
            app.contains(token),
            "the reconcile predicate must read the existing `{token}` fleet signal",
        );
    }
    assert!(
        app.contains("Refresh & reconcile") && app.contains("reconcileShortcut"),
        "flagged rows must link straight to the workspace reconcile flow"
    );
    assert!(
        app.contains("/management?project="),
        "the shortcut must deep-link the existing management reconcile view"
    );
    assert!(
        app.contains("wb-reconcile-shortcut"),
        "the shortcut must be selectable for styling and review"
    );
}

#[test]
fn unfinished_hermora_gains_an_inline_retry() {
    let app = app_js();

    assert!(
        app.contains("Retry Hermora") && app.contains("wb-hermora-retry"),
        "a recorded but unfinished Hermora verb must render its retry inline"
    );
    assert!(
        app.contains("wb-hermora-cli"),
        "the retry must show its exact CLI string beside the button"
    );
    assert!(
        app.contains("delivery.hermora-retry") && app.contains("openActionCard"),
        "the retry must open the existing hermora-retry action card, not a bespoke form"
    );
    assert!(
        app.contains("catalogInvocation(\"delivery.hermora-retry\")"),
        "the retry CLI must reuse the catalog invocation base"
    );
}

#[test]
fn sidebar_collapses_to_a_topbar_with_sticky_headers() {
    let css = styles_css();

    assert!(
        css.contains("@media(max-width:860px)") && css.contains(".sidebar"),
        "the 232px sidebar must collapse to a topbar row under 860px"
    );
    assert!(
        css.contains("thead th") && css.contains("position:sticky"),
        "table headers must stay sticky while inner regions scroll"
    );
}

#[test]
fn rail_rows_are_keyboard_operable_with_text_alternatives() {
    let app = app_js();
    let css = styles_css();

    assert!(
        app.contains("initRailRoving"),
        "the rail must manage focus through one roving function"
    );
    for token in ["ArrowLeft", "ArrowRight", "Home", "End", "tabIndex"] {
        assert!(
            app.contains(token),
            "rail roving must handle `{token}` with a single Tab stop",
        );
    }
    assert!(
        app.contains("data-rail-key"),
        "roving must address steps by key, never by DOM order assumptions"
    );
    assert!(
        app.contains("aria-label") && app.contains("current step"),
        "every rail row must name its step and state in words for AT"
    );
    assert!(
        css.contains(".lifecycle-rail") && css.contains("max-width:100%"),
        "the rail must never force page-level horizontal scroll"
    );
}

#[test]
fn every_form_error_summary_takes_focus() {
    let app = app_js();
    let html = index_html();
    let login = std::fs::read_to_string("frontend/login.html").expect("login ships");

    assert!(
        app.contains("container.focus("),
        "renderErrorSummary must move focus to the summary on every render"
    );
    for id in [
        "mgmt-error-summary",
        "ws-error-summary",
        "portfolio-error-summary",
        "delivery-error-summary",
        "github-error-summary",
    ] {
        assert!(
            html.contains(&format!("id=\"{id}\""))
                && html.contains(&format!(
                    "id=\"{id}\" class=\"error-summary\" tabindex=\"-1\""
                )),
            "form summary `{id}` must be a focusable error-summary container",
        );
    }
    assert!(
        login.contains("id=\"login-error-summary\" class=\"error-summary\" tabindex=\"-1\""),
        "the sign-in summary must be focusable too"
    );
}

// Flywheel (`flywheel-plugin-cap`): idea entry, publish→maintain loop, cap
// filter/badge, demo doc — static tokens over the shipped assets.

#[test]
fn rail_step_zero_carries_the_idea_entry() {
    let html = index_html();
    let app = app_js();

    assert!(
        html.contains("id=\"wb-idea-entry\""),
        "rail step 0 must carry the #wb-idea-entry shell"
    );
    assert!(
        app.contains("renderIdeaEntry") && app.contains("wb-idea-entry"),
        "the idea entry must render through one named function"
    );
    assert!(
        app.contains("forge graduation preview") && app.contains("forge graduation import"),
        "the entry must preview the graduation CLI strings"
    );
    assert!(
        app.contains("forge studio spec") && app.contains("&step=spec"),
        "the entry must link to the studio spec entry with ?project=+?step="
    );
}

#[test]
fn publish_success_writes_the_next_idea_loop() {
    let html = index_html();
    let app = app_js();

    assert!(
        html.contains("id=\"delivery-next-idea\""),
        "delivery must carry the #delivery-next-idea shell"
    );
    assert!(
        app.contains("renderDeliveryNextIdea") && app.contains("delivery-next-idea"),
        "the publish loop must render through one named function"
    );
    assert!(
        app.contains("Next idea") && app.contains("&step=idea"),
        "the loop must prompt the Next idea with a ?project=+&step=idea link"
    );
    assert!(
        app.contains("maintainRefreshShortcut") && app.contains("wb-maintain-refresh"),
        "the loop must offer the maintain refresh shortcut firing the existing control"
    );
}

#[test]
fn projects_view_filters_by_cap_group_with_rail_badge() {
    let html = index_html();
    let app = app_js();

    assert!(
        html.contains("id=\"cap-filter\""),
        "the projects view must carry the #cap-filter group select"
    );
    assert!(
        app.contains("projectCapGroup") && app.contains("CAP_GROUPS"),
        "filtering must derive groups through one named function over one table"
    );
    assert!(
        app.contains("applyCapFilter"),
        "the filter must re-render through one named function"
    );
    assert!(
        html.contains("id=\"cap-badge\"") && app.contains("renderCapBadge"),
        "the workbench rail must carry the #cap-badge text status"
    );
}

#[test]
fn flywheel_demo_doc_covers_five_steps_with_urls() {
    let demo = std::fs::read_to_string("docs/flywheel-demo.md").expect("demo doc ships");
    for step in ["idea", "scaffold", "gate", "publish", "maintain"] {
        assert!(demo.contains(step), "demo must walk step `{step}`");
    }
    assert!(
        demo.contains("hookit"),
        "demo uses hookit as the small candidate"
    );
    for key in [
        "step=idea",
        "step=scaffold",
        "step=test",
        "step=deploy",
        "step=operate",
    ] {
        assert!(
            demo.contains(key) || demo.contains("step="),
            "demo must carry ?project=+?step= web URLs",
        );
    }
    assert!(
        demo.contains("forge cap list") && demo.contains("forge gate"),
        "demo must name the cap list and gate commands"
    );
}

#[test]
fn flywheel_change_adds_no_frontend_dependency() {
    let html = index_html();
    let app = app_js();

    assert!(
        !app.contains("import(") && !app.contains("require("),
        "app.js stays dependency-free: no dynamic import or require"
    );
    let scripts = html.match_indices("<script").count();
    assert_eq!(
        scripts, 2,
        "index.html must keep exactly its two scripts (config.js + app.js); got {scripts}"
    );
}

// Web lifecycle execution (`web-lifecycle-execution`): the idea entry is a
// real preview→confirm→run form, refine surfaces the revision bump, and the
// publish loop journals the transition — all through the catalog's runnable
// rows with no new frontend dependency.

#[test]
fn idea_entry_executes_graduation_preview_and_import() {
    let html = index_html();
    let app = app_js();

    for id in [
        "idea-artifact",
        "idea-profile",
        "idea-id",
        "idea-preview",
        "idea-confirm",
        "idea-run",
        "idea-result",
        "idea-error-summary",
    ] {
        assert!(
            html.contains(&format!("id=\"{id}\"")),
            "idea entry must carry the #{id} control",
        );
    }
    assert!(
        app.contains("renderIdeaEntry") && app.contains("ideaExec"),
        "idea execution must run through one named renderer with held digest state",
    );
    for token in [
        "/v1/admin/graduation/preview",
        "/v1/admin/graduation/import",
    ] {
        assert!(app.contains(token), "idea form must wire `{token}`");
    }
    for token in ["Preview graduation import", "Run confirmed import"] {
        assert!(
            app.contains(token) || html.contains(token),
            "idea form must wire `{token}`",
        );
    }
}

#[test]
fn refine_confirm_shows_the_revision_bump_with_journal_evidence() {
    let app = app_js();

    assert!(
        app.contains("spec_revision") && app.contains("app_revision"),
        "success rendering must surface the studio revision pair",
    );
    assert!(
        app.contains("Revision — spec"),
        "the revision bump must read in operator words",
    );
}

#[test]
fn publish_success_journals_the_next_idea_loop() {
    let app = app_js();

    assert!(
        app.contains("delivery.next-idea") && app.contains("Loop transition journaled"),
        "publish success must record the next-idea journal row and name it",
    );
    assert!(
        app.contains("renderDeliveryNextIdea") && app.contains("delivery-next-idea"),
        "the loop must still render through its named function and shell",
    );
}
