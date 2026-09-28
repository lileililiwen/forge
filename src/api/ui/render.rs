//! Maud templates for the in-process portal UI.
//!
//! Every page is a pure function from owned-string data to a
//! `String`. Maud templates accept owned `String`/`&str` values
//! directly through `{value}` interpolation; project-controlled
//! strings are escaped by default. The only trusted raw string
//! in the tree is the inline `<style>` block.

use maud::{html, Markup, PreEscaped, DOCTYPE};

use super::data::{
    FleetRow, JournalRowView, OperationIdentity, ProjectIdentity, PublishPlanStep, SkippedRow,
};

// --- shared chrome ----------------------------------------------------

pub fn style_sheet() -> PreEscaped<&'static str> {
    PreEscaped(
        "<style>\
:root { color-scheme: light dark; }\
html, body { margin: 0; padding: 0; font-family: ui-sans-serif, system-ui, sans-serif; line-height: 1.4; }\
header { padding: 1rem 1.25rem; border-bottom: 1px solid #ccc; }\
header h1 { margin: 0; font-size: 1.05rem; }\
main { padding: 1rem 1.25rem; }\
footer { padding: 0.5rem 1.25rem; border-top: 1px solid #ccc; color: #666; }\
table { border-collapse: collapse; width: 100%; }\
th, td { border-bottom: 1px solid #ddd; padding: 0.4rem 0.6rem; text-align: left; vertical-align: top; }\
th { background: #f5f5f5; }\
.row-ok { color: #0a7a0a; }\
.row-fail { color: #a02020; }\
.row-warn { color: #8a6a00; }\
.row-skip { color: #666; }\
pre { background: #f5f5f5; padding: 0.5rem; overflow-x: auto; }\
form { margin: 0.5rem 0; }\
button { padding: 0.3rem 0.7rem; }\
nav a { margin-right: 0.5rem; }\
.field-row { margin: 0.25rem 0; }\
.field-label { display: inline-block; min-width: 9rem; color: #444; }\
@media (prefers-color-scheme: dark) { th { background: #2a2a2a; } pre { background: #2a2a2a; } }\
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

fn fleet_frame(title: &str, contract: &str, body: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            (head(title))
            body {
                header {
                    h1 { "Forge fleet" }
                    nav {
                        a href="/ui" { "Refresh" }
                        " · "
                        a href="/healthz" { "API health" }
                    }
                }
                (body)
                footer { small { "forge api ui · " (contract) } }
            }
        }
    }
}

fn project_frame(title: &str, project_id: &str, contract: &str, body: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            (head(title))
            body {
                header {
                    h1 { "Project — " (project_id) }
                    nav {
                        a href="/ui" { "Fleet" }
                        " · "
                        a href={ "/ui/projects/" (project_id) } { "Refresh" }
                    }
                }
                (body)
                footer { small { "forge api ui · " (contract) } }
            }
        }
    }
}

// --- pages ------------------------------------------------------------

pub fn fleet_list(
    title: &str,
    rows: &[FleetRow],
    skipped: &[SkippedRow],
    contract: &str,
) -> String {
    let body = fleet_list_body(rows, skipped);
    fleet_frame(title, contract, body).into_string()
}

fn fleet_list_body(rows: &[FleetRow], skipped: &[SkippedRow]) -> Markup {
    html! {
        main {
            h2 { "Fleet" }
            @if rows.is_empty() && skipped.is_empty() {
                p { "no projects in the fleet registry" }
            } @else {
                table {
                    thead {
                        tr {
                            th { "Project" }
                            th { "Profile" }
                            th { "Last publish" }
                            th { "State" }
                            th { "Subdomain" }
                        }
                    }
                    tbody {
                        @for row in rows {
                            (fleet_row(row))
                        }
                    }
                }
                @if !skipped.is_empty() {
                    h2 { "Skipped" }
                    table {
                        thead {
                            tr {
                                th { "Project" }
                                th { "Classification" }
                                th { "Reason" }
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
                }
                p { small { "Liveness per row lives behind " code { "forge fleet online" } "; the browser UI shows the latest journal state so no target access is required." } }
            }
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
    let token = args.token;
    let origin = args.origin;
    html! {
        main {
            h2 { "Project — " (identity.id) }
            div class="field-row" { span class="field-label" { "Profile" } (identity.profile) }
            div class="field-row" { span class="field-label" { "Maturity" } (identity.maturity) }
            div class="field-row" { span class="field-label" { "Manifest" } code { (identity.manifest_path) } }
            div class="field-row" { span class="field-label" { "Doctor" } (doctor_summary(doctor)) }
            @if !inventory_subdomain.is_empty() {
                div class="field-row" { span class="field-label" { "Subdomain" } (inventory_subdomain) }
            }

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
                table {
                    thead {
                        tr {
                            th { "When" }
                            th { "Kind" }
                            th { "State" }
                            th { "Detail" }
                        }
                    }
                    tbody {
                        @for row in journal {
                            (journal_row(row))
                        }
                    }
                }
            }
        }
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
        main {
            h2 { "Republish — " (project_id) }
            p { "Dry-run plan. Nothing is enqueued until you confirm." }
            @if let Some(pre) = precondition {
                p class="row-fail" { (pre) }
            }
            @if steps.is_empty() {
                p { "no stages would run for this plan." }
            } @else {
                table {
                    thead {
                        tr {
                            th { "Stage" }
                            th { "Status" }
                            th { "Note" }
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
}

pub fn operation_accepted(
    title: &str,
    project_id: &str,
    operation: &OperationIdentity,
    contract: &str,
) -> String {
    let body = html! {
        main {
            h2 { "Republish — " (project_id) }
            p class="row-ok" { "Republish enqueued." }
            div class="field-row" { span class="field-label" { "Operation id" } code { (operation.op_id) } }
            div class="field-row" { span class="field-label" { "Revision" } code { (operation.revision) } }
            div class="field-row" { span class="field-label" { "Container identity" } code { (operation.container_identity) } }
            div class="field-row" { span class="field-label" { "State" } span class="row-warn" { "pending" } }
            p { "Track at " a href={ "/ui/projects/" (project_id) } { "/ui/projects/" (project_id) } " or via " code { "forge deploy status" } "." }
            p { a href="/ui" { "← Back to fleet" } }
        }
    };
    project_frame(title, project_id, contract, body).into_string()
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
    html! {
        (DOCTYPE)
        html lang="en" {
            (head(title))
            body {
                header {
                    h1 { (error_heading(kind)) }
                }
                (body)
                footer { small { "forge api ui · " (contract) } }
            }
        }
    }
    .into_string()
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
        main {
            p class="row-fail" { "code: " code { (code) } }
            p { (message) }
            @if matches!(kind, ErrorKind::Auth) {
                p { "Provide a session id through " code { "Authorization: Bearer <id>" } " or include it in the form field " code { "token" } "." }
            }
            p { a href="/ui" { "← Back to fleet" } }
        }
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
