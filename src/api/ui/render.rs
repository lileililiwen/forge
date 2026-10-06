//! Maud templates for the in-process portal UI.
//!
//! Every page is a pure function from owned-string data to a
//! `String`. Maud templates accept owned `String`/`&str` values
//! directly through `{value}` interpolation; project-controlled
//! strings are escaped by default. The only trusted raw string
//! in the tree is the inline `<style>` block.

use maud::{html, Markup, PreEscaped, DOCTYPE};

use crate::portfolio::{Confidence, EvidenceStatus, Lifecycle, PortfolioFilter};

use super::data::{
    DeliverySummaryView, EvidenceRowView, FleetRow, JournalRowView, OperationIdentity,
    PortfolioEdit, PortfolioRelationView, ProjectIdentity, ProjectPortfolioView, PublishPlanStep,
    ReviewRowView, SkippedRow,
};
use crate::registry::OperationEntry;
use crate::studio::{PreviewEnvelope, StudioSession};

// --- shared chrome ----------------------------------------------------

pub fn style_sheet() -> PreEscaped<&'static str> {
    // The inline stylesheet is the one trusted raw string in the
    // tree. Layout is fluid and token-driven so every page family
    // reflows at 320 CSS px, honours the system light/dark
    // preference, keeps visible focus, and meets the WCAG 2.2 AA
    // contrast/target thresholds. No client JavaScript is used.
    PreEscaped(
        "<style>\
:root {\
  color-scheme: light dark;\
  --surface: #ffffff;\
  --surface-alt: #f4f5f7;\
  --text: #16181a;\
  --muted: #565c63;\
  --border: #8a9199;\
  --link: #0b4fbf;\
  --focus: #b3490a;\
  --ok: #0a6b23;\
  --warn: #7a5400;\
  --fail: #b00020;\
  --skip: #565c63;\
  --min-target: 1.5rem;\
  --primary-target: 2.75rem;\
  --radius: 0.4rem;\
  --max-width: 72rem;\
}\
@media (prefers-color-scheme: dark) {\
  :root {\
    --surface: #101214;\
    --surface-alt: #1c2024;\
    --text: #f3f5f7;\
    --muted: #b6bdc5;\
    --border: #6b737b;\
    --link: #93c0ff;\
    --focus: #ffb066;\
    --ok: #74d68f;\
    --warn: #eec24d;\
    --fail: #ff9d9d;\
    --skip: #b6bdc5;\
  }\
}\
*, *::before, *::after { box-sizing: border-box; }\
html { -webkit-text-size-adjust: 100%; }\
html, body { margin: 0; padding: 0; font-family: ui-sans-serif, system-ui, sans-serif; line-height: 1.5; background: var(--surface); color: var(--text); }\
body { min-height: 100vh; }\
a { color: var(--link); }\
a:hover { text-decoration-thickness: 0.14em; }\
:focus-visible { outline: 3px solid var(--focus); outline-offset: 2px; }\
.skip-link { position: absolute; left: 0.5rem; top: -4rem; z-index: 100; padding: 0.5rem 0.75rem; background: var(--surface-alt); color: var(--text); border: 2px solid var(--focus); border-radius: var(--radius); }\
.skip-link:focus { top: 0.5rem; }\
header { display: flex; flex-wrap: wrap; gap: 0.4rem 1.25rem; align-items: baseline; padding: 1rem 1.25rem; border-bottom: 1px solid var(--border); background: var(--surface-alt); }\
header h1 { margin: 0; font-size: 1.2rem; }\
nav { display: flex; flex-wrap: wrap; gap: 0.4rem 1rem; }\
nav a { white-space: nowrap; }\
main { width: min(100% - 1.5rem, var(--max-width)); margin-inline: auto; padding: 1rem 0 2.5rem; }\
footer { width: min(100% - 1.5rem, var(--max-width)); margin-inline: auto; padding: 1rem 0; border-top: 1px solid var(--border); color: var(--muted); }\
h1, h2, h3, h4 { line-height: 1.25; margin: 1.25rem 0 0.5rem; }\
h2 { font-size: 1.35rem; }\
h3 { font-size: 1.15rem; }\
h4 { font-size: 1rem; }\
p { margin: 0.5rem 0; }\
ul, ol { padding-left: 1.4rem; }\
.field-row { margin: 0.3rem 0; overflow-wrap: anywhere; }\
.field-label { display: inline-block; min-width: 9rem; color: var(--muted); font-weight: 600; }\
.table-scroll { max-width: 100%; overflow-x: auto; border: 1px solid var(--border); border-radius: var(--radius); }\
table { border-collapse: collapse; width: 100%; }\
caption { text-align: left; padding: 0.5rem 0.6rem; color: var(--muted); font-size: 0.9rem; }\
th, td { border-bottom: 1px solid var(--border); padding: 0.45rem 0.6rem; text-align: left; vertical-align: top; }\
th { background: var(--surface-alt); font-weight: 600; }\
.row-ok { color: var(--ok); }\
.row-fail { color: var(--fail); }\
.row-warn, .row-stale { color: var(--warn); }\
.row-skip { color: var(--skip); }\
pre { background: var(--surface-alt); border: 1px solid var(--border); border-radius: var(--radius); padding: 0.5rem; overflow-x: auto; }\
code { overflow-wrap: anywhere; }\
.visually-hidden { position: absolute !important; width: 1px; height: 1px; margin: -1px; padding: 0; overflow: hidden; clip: rect(0 0 0 0); clip-path: inset(50%); white-space: nowrap; border: 0; }\
form { margin: 0.75rem 0; }\
fieldset { margin: 0.75rem 0; border: 1px solid var(--border); border-radius: var(--radius); padding: 0.75rem; }\
legend { font-weight: 600; padding: 0 0.25rem; }\
label { display: inline-block; margin: 0 0.9rem 0.6rem 0; }\
input, select, textarea { font: inherit; color: var(--text); background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 0.35rem 0.5rem; min-height: var(--min-target); min-width: var(--min-target); max-width: 100%; }\
input[type=\"text\"], input[type=\"url\"], select { width: min(22rem, 100%); }\
button, input[type=\"submit\"] { font: inherit; color: var(--surface); background: var(--link); border: 1px solid var(--link); border-radius: var(--radius); padding: 0.5rem 0.95rem; min-height: var(--primary-target); min-width: var(--min-target); cursor: pointer; }\
button:hover, input[type=\"submit\"]:hover { opacity: 0.92; }\
.tag { display: inline-block; margin: 0 0.25rem 0.25rem 0; padding: 0.1rem 0.45rem; border: 1px solid var(--border); border-radius: 0.6rem; font-size: 0.85rem; }\
@media (max-width: 40rem) {\
  header { flex-direction: column; align-items: flex-start; }\
  .field-label { display: block; min-width: 0; }\
  main, footer { width: calc(100% - 1.25rem); }\
}\
@media (prefers-reduced-motion: reduce) {\
  *, *::before, *::after { animation-duration: 0.001ms !important; animation-iteration-count: 1 !important; transition-duration: 0.001ms !important; scroll-behavior: auto !important; }\
}\
</style>",
    )
}

fn head(title: &str) -> Markup {
    html! {
        head {
            meta charset="utf-8" {}
            meta name="viewport" content="width=device-width,initial-scale=1" {}
            title { (title) }
            (style_sheet())
        }
    }
}

/// The one document shell every portal page shares. It guarantees
/// the accessibility structure the contract tests assert: a
/// language-tagged document, a skip link, one labelled `<header>`
/// with a named navigation landmark, exactly one
/// `<main id="main-content">` (the skip link target), and one
/// `<footer>`. Page bodies supply only their inner content; the
/// page-level `<h1>` is always the header heading so a page never
/// renders two level-one headings.
fn chrome(title: &str, contract: &str, heading: &str, nav: Markup, body: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            (head(title))
            body {
                a class="skip-link" href="#main-content" { "Skip to main content" }
                header {
                    h1 { (heading) }
                    nav aria-label="Primary" { (nav) }
                }
                main id="main-content" { (body) }
                footer { small { "forge api ui · " (contract) } }
            }
        }
    }
}

fn fleet_frame(title: &str, contract: &str, body: Markup) -> Markup {
    chrome(
        title,
        contract,
        "Forge fleet",
        html! {
            a href="/ui" { "Refresh" }
            " · "
            a href="/healthz" { "API health" }
        },
        body,
    )
}

fn project_frame(title: &str, project_id: &str, contract: &str, body: Markup) -> Markup {
    chrome(
        title,
        contract,
        &format!("Project — {project_id}"),
        html! {
            a href="/ui" { "Fleet" }
            " · "
            a href={ "/ui/projects/" (project_id) } { "Refresh" }
        },
        body,
    )
}

/// Place a data table inside a named, keyboard-focusable scroll
/// region. The table keeps its own `<caption>`; the region label
/// tells a screen-reader user which table they have entered when
/// the table has to scroll horizontally on narrow screens.
fn table_scroll(label: &str, table: Markup) -> Markup {
    html! {
        div class="table-scroll" role="region" aria-label=(label) tabindex="0" {
            (table)
        }
    }
}

// --- pages ------------------------------------------------------------

pub fn fleet_list(
    title: &str,
    rows: &[FleetRow],
    skipped: &[SkippedRow],
    filter: &PortfolioFilter,
    contract: &str,
) -> String {
    let body = fleet_list_body(rows, skipped, filter);
    fleet_frame(title, contract, body).into_string()
}

/// The active filter rendered above the table. Every control is a
/// plain GET form against `/ui`, so filtering needs no script and
/// no mutation: the portfolio metadata is user-owned, but reading
/// it here writes nothing.
fn portfolio_filter_form(filter: &PortfolioFilter) -> Markup {
    html! {
        form method="get" action="/ui" {
            fieldset {
                legend { "Portfolio filter" }
                label { "Tag " input type="text" name="tag" value=(filter.tag.clone().unwrap_or_default()) {} }
                label {
                    "Lifecycle "
                    select name="lifecycle" {
                        option value="" selected=(filter.lifecycle.is_none()) { "any" }
                        @for value in Lifecycle::ALL {
                            option value=(value.label()) selected=(filter.lifecycle == Some(value)) { (value.label()) }
                        }
                    }
                }
                label {
                    "Confidence "
                    select name="confidence" {
                        option value="" selected=(filter.confidence.is_none()) { "any" }
                        @for value in Confidence::ALL {
                            option value=(value.label()) selected=(filter.confidence == Some(value)) { (value.label()) }
                        }
                    }
                }
                button type="submit" { "Apply" }
                a href="/ui" { "Clear" }
            }
            @if !filter.is_empty() {
                p { small { "Filtered by " (crate::portfolio::filter_query(filter)) "." } }
            }
        }
    }
}

fn fleet_list_body(rows: &[FleetRow], skipped: &[SkippedRow], filter: &PortfolioFilter) -> Markup {
    html! {
        h2 { "Fleet" }
        (portfolio_filter_form(filter))
        @if rows.is_empty() {
            @if filter.is_empty() && skipped.is_empty() {
                p { "no projects in the fleet registry" }
            } @else {
                p { "no project matches the active portfolio filter" }
            }
        } @else {
            (table_scroll("Fleet projects", html! {
                table {
                    caption class="visually-hidden" { "Fleet projects and their latest state" }
                    thead {
                        tr {
                            th scope="col" { "Project" }
                            th scope="col" { "Profile" }
                            th scope="col" { "Lifecycle" }
                            th scope="col" { "Confidence" }
                            th scope="col" { "Tags" }
                            th scope="col" { "Evidence" }
                            th scope="col" { "Last publish" }
                            th scope="col" { "State" }
                            th scope="col" { "Subdomain" }
                        }
                    }
                    tbody {
                        @for row in rows {
                            (fleet_row(row))
                        }
                    }
                }
            }))
            @if !skipped.is_empty() {
                h2 { "Skipped" }
                (table_scroll("Skipped projects", html! {
                    table {
                        caption class="visually-hidden" { "Projects skipped during fleet loading" }
                        thead {
                            tr {
                                th scope="col" { "Project" }
                                th scope="col" { "Classification" }
                                th scope="col" { "Reason" }
                            }
                        }
                        tbody {
                            @for row in skipped {
                                tr {
                                    td { code { (row.id) } }
                                    td { (row.classification) }
                                    td { (row.reason) }
                                }
                            }
                        }
                    }
                }))
            }
            p { small { "Liveness per row lives behind " code { "forge fleet online" } "; the browser UI shows the latest journal state so no target access is required." } }
            p { small { "Lifecycle, confidence and tags are user-owned portfolio metadata (" code { "forge portfolio" } "). Evidence states are imported from their own source systems; an " code { "unavailable" } " or " code { "stale" } " row never means the check passed." } }
        }
    }
}

fn fleet_row(row: &FleetRow) -> Markup {
    let detail_href = format!("/ui/projects/{}", row.id);
    let state_class = state_class(&row.last_state);
    html! {
        tr {
            td {
                a href={ (detail_href) } { code { (row.id) } }
            }
            td { (row.profile) }
            td {
                @if let Some(lifecycle) = row.lifecycle {
                    (lifecycle.label())
                } @else {
                    span class="row-skip" { "—" }
                }
            }
            td {
                @if let Some(confidence) = row.confidence {
                    (confidence.label())
                } @else {
                    span class="row-skip" { "—" }
                }
            }
            td {
                @if row.tags.is_empty() {
                    span class="row-skip" { "—" }
                } @else {
                    @for tag in &row.tags {
                        span class="tag" { (tag) }
                    }
                }
            }
            td {
                @if row.evidence.is_empty() {
                    span class="row-skip" { "no evidence" }
                } @else {
                    @for (source, status) in &row.evidence {
                        div { (source) " " span class={ (evidence_class(*status)) } { (status.label()) } }
                    }
                }
            }
            td { (row.last_at) }
            td {
                span class={ (state_class) } { (row.last_state) }
            }
            td {
                @if let Some(ref sub) = row.subdomain {
                    a href={ "https://" (sub) } { (sub) }
                } @else {
                    "—"
                }
            }
        }
    }
}

pub struct ProjectDetailArgs<'a> {
    pub title: &'a str,
    pub identity: &'a ProjectIdentity,
    pub doctor: &'a DoctorSummary,
    pub inventory_subdomain: &'a str,
    pub journal: &'a [JournalRowView],
    pub portfolio: &'a ProjectPortfolioView,
    pub delivery: &'a DeliverySummaryView,
    pub token: &'a str,
    pub origin: &'a str,
    pub contract: &'a str,
}

#[allow(clippy::too_many_arguments)]
pub fn project_detail(args: ProjectDetailArgs<'_>) -> String {
    let ProjectDetailArgs {
        title,
        identity,
        contract,
        ..
    } = args;
    let body = project_detail_body(args);
    project_frame(title, &identity.id, contract, body).into_string()
}

fn project_detail_body(args: ProjectDetailArgs<'_>) -> Markup {
    let identity = args.identity;
    let doctor = args.doctor;
    let inventory_subdomain = args.inventory_subdomain;
    let journal = args.journal;
    let portfolio = args.portfolio;
    let delivery = args.delivery;
    let token = args.token;
    let origin = args.origin;
    html! {
        h2 { "Project — " (identity.id) }
        div class="field-row" { span class="field-label" { "Profile" } (identity.profile) }
        div class="field-row" { span class="field-label" { "Maturity" } (identity.maturity) }
        div class="field-row" { span class="field-label" { "Manifest" } code { (identity.manifest_path) } }
        div class="field-row" { span class="field-label" { "Doctor" } (doctor_summary(doctor)) }
        @if !inventory_subdomain.is_empty() {
            div class="field-row" { span class="field-label" { "Subdomain" } (inventory_subdomain) }
        }

        (delivery_section(delivery))

        (portfolio_section(portfolio, identity, token, origin))

        h3 { "Republish" }
        p { "Dry-run preview, then confirm. The publish enqueues the same " code { "RemoteComposeAdapter" } " lane as " code { "forge publish all" } "." }
        form method="post" action={ "/ui/projects/" (identity.id) "/publish" } {
            input type="hidden" name="token" value={ (token) } {}
            input type="hidden" name="origin" value={ (origin) } {}
            button type="submit" { "Preview plan" }
        }

        h3 { "Recent operations" }
        @if journal.is_empty() {
            p { "no journal rows recorded for this project" }
        } @else {
            (table_scroll("Recent operations", html! {
                table {
                    caption class="visually-hidden" { "Recent operations for this project" }
                    thead {
                        tr {
                            th scope="col" { "When" }
                            th scope="col" { "Kind" }
                            th scope="col" { "State" }
                            th scope="col" { "Detail" }
                        }
                    }
                    tbody {
                        @for row in journal {
                            (journal_row(row))
                        }
                    }
                }
            }))
        }
    }
}

/// The user-owned portfolio block plus its write form.
///
/// The section separates the two ownership domains explicitly: the
/// classification fields are editable here, while the evidence
/// table is read-only and attributes every row to the source
/// system and revision that produced it. No form on this page
/// writes a repository file or a provider record.
fn portfolio_section(
    portfolio: &ProjectPortfolioView,
    identity: &ProjectIdentity,
    token: &str,
    origin: &str,
) -> Markup {
    html! {
        h3 { "Portfolio" }
        div class="field-row" { span class="field-label" { "Lifecycle" } (portfolio.lifecycle) }
        div class="field-row" { span class="field-label" { "Confidence" } (portfolio.confidence) }
        div class="field-row" { span class="field-label" { "Next action" } (portfolio.next_action) }
        div class="field-row" {
            span class="field-label" { "Blocker" }
            @if portfolio.blocker == "—" {
                span class="row-skip" { "—" }
            } @else {
                span class="row-warn" { (portfolio.blocker) }
            }
        }
        div class="field-row" { span class="field-label" { "Reviewed at" } (portfolio.reviewed_at) }
        div class="field-row" {
            span class="field-label" { "Tags" }
            @if portfolio.tags.is_empty() {
                span class="row-skip" { "—" }
            } @else {
                @for tag in &portfolio.tags {
                    span class="tag" { (tag) }
                }
            }
        }
        @if !portfolio.goals.is_empty() {
            div class="field-row" {
                span class="field-label" { "Goals" }
                @for (title, status) in &portfolio.goals {
                    span { (title) " " small { "[" (status) "]" } " " }
                }
            }
        }

        h4 { "Relations" }
        @if portfolio.relations.is_empty() {
            p { "no declared relation for this project" }
        } @else {
            (table_scroll("Declared relations", html! {
                table {
                    caption class="visually-hidden" { "Relations declared for this project" }
                    thead {
                        tr {
                            th scope="col" { "Direction" }
                            th scope="col" { "Relation" }
                            th scope="col" { "Project" }
                            th scope="col" { "Note" }
                        }
                    }
                    tbody {
                        @for relation in &portfolio.relations {
                            (relation_row(relation))
                        }
                    }
                }
            }))
        }

        h4 { "Imported evidence" }
        p { small { "Observations are imported from their own source systems. Forge stores what the source reported; it does not run the check." } }
        @if portfolio.evidence.is_empty() {
            p { "no source observation imported for this project" }
        } @else {
            (table_scroll("Imported evidence", html! {
                table {
                    caption class="visually-hidden" { "Evidence imported for this project" }
                    thead {
                        tr {
                            th scope="col" { "Source" }
                            th scope="col" { "Source revision" }
                            th scope="col" { "Observed at" }
                            th scope="col" { "Stale after" }
                            th scope="col" { "State" }
                        }
                    }
                    tbody {
                        @for row in &portfolio.evidence {
                            (evidence_row(row))
                        }
                    }
                }
            }))
        }

        h4 { "Review history" }
        @if portfolio.reviews.is_empty() {
            p { "no review recorded for this project" }
        } @else {
            (table_scroll("Review history", html! {
                table {
                    caption class="visually-hidden" { "Review history for this project" }
                    thead {
                        tr {
                            th scope="col" { "When" }
                            th scope="col" { "Confidence" }
                            th scope="col" { "Note" }
                        }
                    }
                    tbody {
                        @for row in &portfolio.reviews {
                            (review_row(row))
                        }
                    }
                }
            }))
        }

        h4 { "Edit portfolio metadata" }
        form method="post" action={ "/ui/projects/" (identity.id) "/portfolio" } {
            input type="hidden" name="token" value={ (token) } {}
            input type="hidden" name="origin" value={ (origin) } {}
            fieldset {
                legend { "Classification" }
                label {
                    "Lifecycle "
                    select name="lifecycle" {
                        option value="" { "unchanged" }
                        @for value in Lifecycle::ALL {
                            option value=(value.label()) { (value.label()) }
                        }
                    }
                }
                label {
                    "Confidence "
                    select name="confidence" {
                        option value="" { "unchanged" }
                        @for value in Confidence::ALL {
                            option value=(value.label()) { (value.label()) }
                        }
                    }
                }
            }
            fieldset {
                legend { "Notes" }
                label { "Next action " input type="text" name="next_action" value="" {} }
                label { "Blocker " input type="text" name="blocker" value="" {} }
            }
            fieldset {
                legend { "Tags" }
                label { "Add " input type="text" name="tag" value="" {} }
                label { "Remove " input type="text" name="remove_tag" value="" {} }
            }
            button type="submit" { "Save portfolio metadata" }
        }
    }
}

fn relation_row(relation: &PortfolioRelationView) -> Markup {
    let href = format!("/ui/projects/{}", relation.other_project);
    html! {
        tr {
            td { (relation.direction) }
            td { code { (relation.relation_type) } }
            td { a href={ (href) } { code { (relation.other_project) } } }
            td { small { (relation.note) } }
        }
    }
}

fn evidence_row(row: &EvidenceRowView) -> Markup {
    html! {
        tr {
            td { code { (row.source_system) } }
            td { code { (row.source_revision) } }
            td { (row.observed_at) }
            td {
                @if row.stale_after.is_empty() {
                    span class="row-skip" { "—" }
                } @else {
                    (row.stale_after)
                }
            }
            td { span class={ (evidence_class(row.status)) } { (row.status.label()) } }
        }
    }
}

fn review_row(row: &ReviewRowView) -> Markup {
    html! {
        tr {
            td { (row.reviewed_at) }
            td { (row.confidence) }
            td { small { (row.note) } }
        }
    }
}

/// Render the applied edits after a confirm-gated portfolio POST.
pub fn portfolio_saved(
    title: &str,
    project_id: &str,
    edits: &[PortfolioEdit],
    contract: &str,
) -> String {
    let body = html! {
        h2 { "Portfolio — " (project_id) }
        p class="row-ok" { "Saved portfolio metadata." }
        ul {
            @for edit in edits {
                li { strong { (edit.action) } " — " (edit.detail) }
            }
        }
        p { "Imported evidence was not touched; source-owned snapshots are append-only." }
        p { a href={ "/ui/projects/" (project_id) } { "← Back to project" } }
        p { a href="/ui" { "← Back to fleet" } }
    };
    project_frame(title, project_id, contract, body).into_string()
}

fn delivery_section(delivery: &DeliverySummaryView) -> Markup {
    html! {
        h3 { "Delivery" }
        div class="field-row" { span class="field-label" { "Phase" } (delivery.phase) }
        @if let Some(env) = &delivery.environment {
            div class="field-row" { span class="field-label" { "Environment" } (env) }
        } @else {
            div class="field-row" { span class="field-label" { "Environment" } span class="row-skip" { "—" } }
        }
        @if let Some(rev) = &delivery.revision {
            div class="field-row" { span class="field-label" { "Revision" } code { (rev) } }
        } @else {
            div class="field-row" { span class="field-label" { "Revision" } span class="row-skip" { "—" } }
        }
        div class="field-row" { span class="field-label" { "Updated" } (delivery.updated_at) }
        div class="field-row" { span class="field-label" { "Preflight" }
            @if let Some(state) = &delivery.preflight_state { (state) } @else { span class="row-skip" { "—" } } }
        div class="field-row" { span class="field-label" { "Stage" }
            @if let Some(state) = &delivery.stage_state { (state) } @else { span class="row-skip" { "—" } } }
        div class="field-row" { span class="field-label" { "Promote" }
            @if let Some(state) = &delivery.promote_state { (state) } @else { span class="row-skip" { "—" } } }
        div class="field-row" { span class="field-label" { "Hermora" }
            @if let Some(state) = &delivery.hermora_state { (state) } @else { span class="row-skip" { "—" } } }
    }
}

fn doctor_summary(summary: &DoctorSummary) -> Markup {
    let class = match summary.kind {
        DoctorKind::Pass => "row-ok",
        DoctorKind::Warn => "row-warn",
        DoctorKind::Fail => "row-fail",
        DoctorKind::Unknown => "row-skip",
    };
    html! {
        span class={ (class) } { (summary.label) }
        @if !summary.detail.is_empty() {
            " — "
            small { (summary.detail) }
        }
    }
}

fn journal_row(row: &JournalRowView) -> Markup {
    let class = state_class(&row.state);
    html! {
        tr {
            td { (row.started_at) }
            td { (row.kind) }
            td { span class={ (class) } { (row.state) } }
            td { small { (row.detail) } }
        }
    }
}

/// Render the read-only Studio page. The page surfaces the
/// saved AppSpec summary, the bounded preview envelope, and the
/// most recent `studio.*` journal rows so the operator can audit
/// what has been reviewed and what the next action is. Interactive
/// controls (prompt form, refinement textarea) land in a follow-up
/// cycle (`tasks.md` §5.2); the current page is intentionally
/// honest about its read-only posture.
pub fn studio_page(
    project_id: &str,
    session: Option<&StudioSession>,
    envelope: &PreviewEnvelope,
    journal_rows: &[OperationEntry],
    contract: &str,
) -> String {
    let spec_summary = session
        .map(|s| {
            format!(
                "name={} profile={} schema_version={} pages={} sections={}",
                s.spec.name,
                s.spec.profile,
                s.spec.schema_version,
                s.spec.pages.len(),
                s.spec.pages.iter().map(|p| p.sections.len()).sum::<usize>(),
            )
        })
        .unwrap_or_else(|| "no spec saved yet".to_string());
    let revisions = session
        .map(|s| {
            format!(
                "spec_revision={} app_revision={}",
                s.spec_revision, s.app_revision
            )
        })
        .unwrap_or_else(|| "spec_revision=- app_revision=-".to_string());
    let preview_url = envelope
        .preview_url
        .clone()
        .unwrap_or_else(|| "(none)".to_string());
    let body = html! {
        (project_frame("Forge studio", project_id, contract, html! {
            section {
                h2 { "AppSpec" }
                p { (spec_summary) }
                p { (revisions) }
                @if let Some(session) = session {
                    h3 { "Pages" }
                    ul {
                        @for page in &session.spec.pages {
                            li { (page.route) " — " (page.title) }
                        }
                    }
                }
            }
            section {
                h2 { "Preview" }
                p { "state=" (envelope.state.label()) }
                p { "port=" (envelope.port.map(|p| p.to_string()).unwrap_or_else(|| "-".to_string())) }
                p { "preview_url=" (preview_url) }
                @if let Some(code) = &envelope.last_error_code {
                    p { "last_error_code=" (code) }
                }
                p { "Interactive preview controls land in a follow-up cycle." }
            }
            section {
                h2 { "Journal" }
                @if journal_rows.is_empty() {
                    p { "No studio.* journal rows yet." }
                } @else {
                    (table_scroll("Studio journal", html! {
                        table {
                            caption class="visually-hidden" { "Recent studio operations" }
                            thead {
                                tr {
                                    th scope="col" { "kind" }
                                    th scope="col" { "state" }
                                    th scope="col" { "started_at" }
                                    th scope="col" { "detail" }
                                }
                            }
                            tbody {
                                @for row in journal_rows {
                                    tr {
                                        td { (row.kind.clone()) }
                                        td { (row.state.clone()) }
                                        td { (row.started_at.clone()) }
                                        td { (row.detail.clone().unwrap_or_else(|| "-".to_string())) }
                                    }
                                }
                            }
                        }
                    }))
                }
            }
        }))
    };
    body.into_string()
}

pub fn publish_plan(
    title: &str,
    project_id: &str,
    steps: &[PublishPlanStep],
    precondition: Option<&str>,
    token: &str,
    origin: &str,
    contract: &str,
) -> String {
    let body = publish_plan_body(project_id, steps, precondition, token, origin);
    project_frame(title, project_id, contract, body).into_string()
}

fn publish_plan_body(
    project_id: &str,
    steps: &[PublishPlanStep],
    precondition: Option<&str>,
    token: &str,
    origin: &str,
) -> Markup {
    html! {
        h2 { "Republish — " (project_id) }
        p { "Dry-run plan. Nothing is enqueued until you confirm." }
        @if let Some(pre) = precondition {
            p class="row-fail" { (pre) }
        }
        @if steps.is_empty() {
            p { "no stages would run for this plan." }
        } @else {
            (table_scroll("Publish plan stages", html! {
                table {
                    caption class="visually-hidden" { "Publish plan stages" }
                    thead {
                        tr {
                            th scope="col" { "Stage" }
                            th scope="col" { "Status" }
                            th scope="col" { "Note" }
                        }
                    }
                    tbody {
                        @for step in steps {
                            tr {
                                td { code { (step.stage) } }
                                td {
                                    span class={ (state_class(&step.status)) } { (step.status) }
                                }
                                td { small { (step.note) } }
                            }
                        }
                    }
                }
            }))
        }
        form method="post" action={ "/ui/projects/" (project_id) "/publish" } {
            input type="hidden" name="token" value={ (token) } {}
            input type="hidden" name="origin" value={ (origin) } {}
            input type="hidden" name="confirm" value="yes" {}
            button type="submit" { "Confirm republish" }
        }
        p { a href={ "/ui/projects/" (project_id) } { "← Back to project" } }
    }
}

pub fn operation_accepted(
    title: &str,
    project_id: &str,
    operation: &OperationIdentity,
    contract: &str,
) -> String {
    let body = html! {
        h2 { "Republish — " (project_id) }
        p class="row-ok" { "Republish enqueued." }
        div class="field-row" { span class="field-label" { "Operation id" } code { (operation.op_id) } }
        div class="field-row" { span class="field-label" { "Revision" } code { (operation.revision) } }
        div class="field-row" { span class="field-label" { "Container identity" } code { (operation.container_identity) } }
        div class="field-row" { span class="field-label" { "State" } span class="row-warn" { "pending" } }
        p { "Track at " a href={ "/ui/projects/" (project_id) } { "/ui/projects/" (project_id) } " or via " code { "forge deploy status" } "." }
        p { a href="/ui" { "← Back to fleet" } }
    };
    project_frame(title, project_id, contract, body).into_string()
}

// --- sign-in / callback / sign-out pages --------------------------------

/// Render the anonymous portal entry point. The operator supplies a project
/// id so Forge does not disclose its registered project inventory before
/// authentication; the existing sign-in route validates the submitted id and
/// performs the configured OIDC round trip.
pub fn sign_in_entry_page(title: &str, return_path: &str, contract: &str) -> String {
    let body = html! {
        h2 { "Choose a project to sign in" }
        p { "Enter a registered project id. Forge will use that project's configured identity provider." }
        form method="get" action="/ui/sign-in" {
            input type="hidden" name="return" value=(return_path) {};
            label for="project-id" { "Project id" }
            input id="project-id" type="text" name="project" required="required" autocomplete="off" {};
            button type="submit" { "Continue" }
        }
    };
    let nav = html! {
        a href="/healthz" { "API health" }
    };
    chrome(title, contract, "Sign in", nav, body).into_string()
}

/// Render the sign-in page. The page is rendered in two
/// shapes: the issuing form on `GET /ui/sign-in?project=…`
/// (a short page that links to the provider) and a
/// follow-up page on the callback handler that explains
/// the failure in redacted terms and points the operator
/// back to the sign-in form. The page never reflects a
/// state value, a code value, or any provider error
/// verbatim.
pub fn sign_in_page(
    title: &str,
    project_id: &str,
    auth_url: &str,
    return_path: &str,
    contract: &str,
) -> String {
    let body = html! {
        @if !project_id.is_empty() {
            h2 { "Continue sign-in" }
            p { "Sign in to " code { (project_id) } " via the configured OIDC provider." }
            p { a href=(auth_url) { "Continue to provider" } }
        } @else {
            h2 { "Signed out" }
            p { "Sign out complete. " a href="/ui/sign-in" { "Sign in again" } "." }
        }
        @if !return_path.is_empty() && return_path != "/ui" {
            p { small { "After sign-in you will return to " code { (return_path) } "." } }
        }
    };
    let nav = html! {
        a href="/ui" { "Fleet" }
        " · "
        a href="/ui/sign-in" { "Sign in" }
    };
    chrome(title, contract, title, nav, body).into_string()
}

// --- error pages ------------------------------------------------------

pub fn error_page(
    title: &str,
    code: &str,
    message: &str,
    kind: ErrorKind,
    contract: &str,
) -> String {
    let body = error_body(code, message, kind);
    let nav = html! {
        a href="/ui" { "Fleet" }
    };
    chrome(title, contract, error_heading(kind), nav, body).into_string()
}

fn error_heading(kind: ErrorKind) -> &'static str {
    match kind {
        ErrorKind::Auth => "Sign in required",
        ErrorKind::Project => "Forge ui",
        ErrorKind::Conflict => "Confirm required",
    }
}

fn error_body(code: &str, message: &str, kind: ErrorKind) -> Markup {
    html! {
        h2 { "What happened" }
        p class="row-fail" { "code: " code { (code) } }
        p { (message) }
        @if matches!(kind, ErrorKind::Auth) {
            p { "Provide a session id through " code { "Authorization: Bearer <id>" } " or include it in the form field " code { "token" } "." }
        }
        p { a href="/ui" { "← Back to fleet" } }
    }
}

fn state_class(state: &str) -> &'static str {
    match state {
        "done" => "row-ok",
        "failed" | "blocked" => "row-fail",
        "pending" => "row-warn",
        _ => "row-skip",
    }
}

/// CSS class for an imported evidence state. Only `observed` reads
/// as a result; `stale` and every absence state stay visually
/// distinct from a pass so an unavailable provider is never
/// mistaken for a green check.
fn evidence_class(status: EvidenceStatus) -> &'static str {
    match status {
        EvidenceStatus::Observed => "row-ok",
        EvidenceStatus::Stale | EvidenceStatus::NotRun => "row-stale",
        EvidenceStatus::Unavailable | EvidenceStatus::Invalid => "row-skip",
    }
}

// --- typed page data (carried by the renderer module for clarity) -----

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoctorKind {
    Pass,
    Warn,
    Fail,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct DoctorSummary {
    pub kind: DoctorKind,
    pub label: String,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Auth,
    Project,
    Conflict,
}
