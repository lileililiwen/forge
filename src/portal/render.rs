//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use chrono::{DateTime, Utc};

use super::model::PortalSectionView;

/// detail line. The renderer prints the section heading,
/// the rolled-up status and one line per entry. The SPA
/// pointer (`portal-spa-convergence`) names the matching
/// dashboard deep-link and the SPA coverage verdict so the
/// legacy HTML reads as a pointer, not a second surface.
pub fn render_section_human(view: &PortalSectionView) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "[{label}] section=`{id}` status=`{status}` source=`{source}` generated_at {ts}\n",
        label = view.section.label(),
        id = view.section_id,
        status = view.status.id(),
        source = view.source,
        ts = view.generated_at,
    ));
    let spa = if view.spa_route.is_empty() {
        "(none — CLI only)".to_string()
    } else {
        view.spa_route.clone()
    };
    let coverage = if view.web_coverage.is_empty() {
        view.section.web_coverage()
    } else {
        view.web_coverage.as_str()
    };
    out.push_str(&format!("  spa: {spa}\n"));
    out.push_str(&format!(
        "  web: {coverage} — {note}\n",
        note = view.section.spa_note()
    ));
    if let Some(id) = view.project_id.as_ref() {
        out.push_str(&format!("  project_id: {id}\n"));
    }
    if view.entries.is_empty() {
        out.push_str("  entries: (none)\n");
    } else {
        out.push_str(&format!("  entries: {n}\n", n = view.entries.len()));
        for entry in &view.entries {
            out.push_str(&format!(
                "  - id=`{id}` status=`{status}` label=`{label}` source=`{source}` observed_at {ts}\n",
                id = entry.id,
                status = entry.status.id(),
                label = entry.label,
                source = entry.source,
                ts = entry.observed_at,
            ));
            for line in &entry.evidence {
                out.push_str(&format!("      evidence: {line}\n"));
            }
            for (key, value) in &entry.attributes {
                out.push_str(&format!("      attr: {key}={value}\n"));
            }
        }
    }
    if !view.controls_available.is_empty() {
        out.push_str(&format!(
            "  controls_available: {}\n",
            view.controls_available.join(", ")
        ));
    }
    out
}

/// RFC 3339 timestamp for the `generated_at` field. The
/// clock is the system clock; the value is stable across
/// every entry in a single view because the caller
/// threads the same `now` through the build pipeline.
pub fn utc_now() -> String {
    let now: DateTime<Utc> = Utc::now();
    now.to_rfc3339()
}
