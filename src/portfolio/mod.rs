//! Portfolio domain: user-owned horizontal project management data.
//!
//! This module owns the *vocabulary* and *validation* of the
//! portfolio read model. It deliberately stores nothing itself:
//! persistence lives beside the project registry
//! ([`crate::registry::portfolio`]) because the two share one
//! SQLite file and one migration lifecycle.
//!
//! The split between the two mirrors the product boundary:
//!
//! - **User-owned** facts — lifecycle, confidence, tags, goals,
//!   relations, blockers, next actions and review history — are
//!   editable through [`crate::registry::portfolio`].
//! - **Source-owned** observations — governance, gate, release and
//!   runtime evidence — arrive only as append-only snapshots
//!   carrying their source system, source revision, observation
//!   time and status. Forge stores what somebody else observed; it
//!   never claims to have performed the check and never rewrites a
//!   snapshot as a source fact.
//!
//! Every read model derived from a snapshot preserves the
//! unavailable and stale states verbatim: an absent provider is
//! `unavailable`, an expired freshness bound is `stale`, and
//! neither is ever reported as healthy or passing.
//!
//! Three sub-packages split the domain by direction rather than by
//! data: [`share`] and [`publication`] decide what may *leave* Forge,
//! while [`interest`] and [`interest_report`] decide what may *enter*
//! it. Neither direction shares a row, a table or a rule with the
//! other.

use serde::Serialize;

use crate::policy::redact_credentials;

pub mod interest;
pub mod interest_report;
pub mod publication;
pub mod share;
pub mod vocabulary;
pub use vocabulary::{Confidence, EvidenceStatus, Lifecycle, RelationType};

/// Contract version for every portfolio document Forge emits.
pub const PORTFOLIO_CONTRACT_VERSION: &str = "forge-portfolio/0.1.0";

/// Upper bound on a stored evidence payload, in bytes. A hostile
/// or runaway provider cannot grow one portfolio row without
/// bound; the snapshot is refused before it is persisted.
pub const MAX_EVIDENCE_BYTES: usize = 64 * 1024;

/// Upper bound on a user-authored tag name, in characters.
pub const MAX_TAG_NAME_CHARS: usize = 64;

/// Upper bound on a user-authored free-text field (blocker, next
/// action, relation note, review note), in characters.
pub const MAX_NOTE_CHARS: usize = 2048;

/// Upper bound on a goal title, in characters.
pub const MAX_GOAL_TITLE_CHARS: usize = 200;

/// Upper bound on a goal description, in characters.
pub const MAX_GOAL_DESCRIPTION_CHARS: usize = 2048;
// --- validation --------------------------------------------------------

/// Validate a free-form tag name. Names are lowercase kebab or
/// single words so a tag is typeable in a URL query without
/// escaping; a future controlled vocabulary layers on top of the
/// same identity rather than replacing it.
pub fn validate_tag_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("tag name must not be empty".to_string());
    }
    if trimmed.chars().count() > MAX_TAG_NAME_CHARS {
        return Err(format!(
            "tag name is longer than {MAX_TAG_NAME_CHARS} characters"
        ));
    }
    let mut prev_dash = false;
    for c in trimmed.chars() {
        if c == '-' {
            if prev_dash {
                return Err(format!(
                    "invalid tag name '{trimmed}': must not contain consecutive dashes"
                ));
            }
            prev_dash = true;
        } else if c.is_ascii_lowercase() || c.is_ascii_digit() {
            prev_dash = false;
        } else {
            return Err(format!(
                "invalid tag name '{trimmed}': use lowercase letters, digits and single dashes"
            ));
        }
    }
    if trimmed.starts_with('-') || trimmed.ends_with('-') {
        return Err(format!(
            "invalid tag name '{trimmed}': must not start or end with a dash"
        ));
    }
    Ok(trimmed.to_string())
}

/// Validate a tag colour. Colours are rendered as a CSS class or a
/// `#rrggbb` value, so anything outside that shape is refused
/// rather than escaped at render time.
pub fn validate_tag_color(raw: Option<&str>) -> Result<Option<String>, String> {
    let Some(value) = raw else {
        return Ok(None);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.len() == 7
        && trimmed.starts_with('#')
        && trimmed[1..].chars().all(|c| c.is_ascii_hexdigit())
    {
        return Ok(Some(trimmed.to_ascii_lowercase()));
    }
    if trimmed.chars().all(|c| c.is_ascii_lowercase()) && !trimmed.is_empty() {
        return Ok(Some(trimmed.to_string()));
    }
    Err(format!(
        "invalid tag colour '{trimmed}': expected a `#rrggbb` hex value or a lowercase word"
    ))
}

/// Validate a bounded free-text note. Control characters are
/// refused so a rendered page can never carry a stray newline or
/// escape sequence into a table cell.
pub fn validate_note(field: &str, raw: &str, max_chars: usize) -> Result<Option<String>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > max_chars {
        return Err(format!("{field} is longer than {max_chars} characters"));
    }
    if trimmed.chars().any(|c| c.is_control()) {
        return Err(format!("{field} must not contain control characters"));
    }
    Ok(Some(trimmed.to_string()))
}

/// Validate a source-system label. A snapshot without an honest
/// source label would let a Forge-local observation masquerade as
/// an external one, so the label is required and bounded.
pub fn validate_source_system(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("evidence import requires a source system".to_string());
    }
    if trimmed.chars().count() > MAX_TAG_NAME_CHARS {
        return Err(format!(
            "source system is longer than {MAX_TAG_NAME_CHARS} characters"
        ));
    }
    let mut prev_dash = false;
    for c in trimmed.chars() {
        if c == '-' || c == '_' || c == '.' {
            if prev_dash {
                return Err(format!(
                    "invalid source system '{trimmed}': must not repeat separators"
                ));
            }
            prev_dash = true;
        } else if c.is_ascii_lowercase() || c.is_ascii_digit() {
            prev_dash = false;
        } else {
            return Err(format!(
                "invalid source system '{trimmed}': use lowercase letters, digits, '-', '_' and '.'"
            ));
        }
    }
    if trimmed.starts_with(['-', '_', '.']) || trimmed.ends_with(['-', '_', '.']) {
        return Err(format!(
            "invalid source system '{trimmed}': must not start or end with a separator"
        ));
    }
    Ok(trimmed.to_string())
}

/// Validate a source revision. Any bounded, control-free label is
/// accepted so a governance report, a release tag and a git SHA
/// can all be attributed without inventing one canonical form.
pub fn validate_source_revision(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("evidence import requires a source revision".to_string());
    }
    if trimmed.chars().count() > 256 {
        return Err("source revision is longer than 256 characters".to_string());
    }
    if trimmed.chars().any(|c| c.is_control()) {
        return Err("source revision must not contain control characters".to_string());
    }
    Ok(trimmed.to_string())
}

/// Validate a goal status. Goals are user-owned, so the vocabulary
/// is small and closed.
pub fn validate_goal_status(raw: &str) -> Result<String, String> {
    const ALLOWED: [&str; 4] = ["planned", "active", "done", "dropped"];
    let trimmed = raw.trim();
    if ALLOWED.contains(&trimmed) {
        Ok(trimmed.to_string())
    } else {
        Err(format!(
            "unknown goal status '{trimmed}'; expected one of {}",
            ALLOWED.join(", ")
        ))
    }
}

/// Prepare an evidence payload for storage: bound its size, parse
/// it as a JSON object so a malformed or scalar payload is
/// rejected rather than stored, and redact every
/// credential-shaped span through the shared
/// [`redact_credentials`] policy.
pub fn prepare_evidence(raw: &str) -> Result<String, String> {
    if raw.len() > MAX_EVIDENCE_BYTES {
        return Err(format!(
            "evidence payload is larger than {MAX_EVIDENCE_BYTES} bytes"
        ));
    }
    match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(serde_json::Value::Object(_)) => Ok(redact_credentials(raw)),
        Ok(_) => Err("evidence payload must be a JSON object".to_string()),
        Err(err) => Err(format!("evidence payload is not valid JSON: {err}")),
    }
}

// --- staleness ---------------------------------------------------------

/// One imported source-owned observation.
#[derive(Debug, Clone, Serialize)]
pub struct EvidenceSnapshot {
    pub snapshot_id: i64,
    pub project_id: String,
    pub source_system: String,
    pub source_revision: String,
    pub observed_at: String,
    pub status: EvidenceStatus,
    /// Freshness bound recorded at import time, if the source
    /// declared one. `None` means the source declared no expiry
    /// and the snapshot stays `observed` until it is replaced.
    pub stale_after: Option<String>,
    /// Redacted evidence document, verbatim as stored.
    pub evidence_json: String,
    /// The state the read model actually displays: the recorded
    /// `status`, downgraded to `stale` once `stale_after` has
    /// passed. Never upgraded, and never `observed` when the
    /// recorded state is not `observed`.
    pub effective_status: EvidenceStatus,
}

/// Decide the displayed state for one snapshot. Expiry can only
/// downgrade `observed` to `stale`; an `unavailable`, `invalid` or
/// `not-run` snapshot stays exactly as the source reported it, and
/// an unparseable bound never invents freshness.
pub fn effective_status(
    status: EvidenceStatus,
    stale_after: Option<&str>,
    now: chrono::DateTime<chrono::Utc>,
) -> EvidenceStatus {
    if status != EvidenceStatus::Observed {
        return status;
    }
    match stale_after.and_then(|value| {
        chrono::DateTime::parse_from_rfc3339(value)
            .ok()
            .map(|bound| bound.with_timezone(&chrono::Utc))
    }) {
        Some(bound) if bound <= now => EvidenceStatus::Stale,
        _ => EvidenceStatus::Observed,
    }
}

/// Compute the effective status of an already-loaded row.
pub fn snapshot_effective_status(
    snapshot: &EvidenceSnapshot,
    now: chrono::DateTime<chrono::Utc>,
) -> EvidenceStatus {
    effective_status(snapshot.status, snapshot.stale_after.as_deref(), now)
}

/// One user-owned tag.
#[derive(Debug, Clone, Serialize)]
pub struct TagRecord {
    pub tag_id: i64,
    pub name: String,
    pub color: Option<String>,
    pub created_at: String,
}

/// One directed project relationship.
#[derive(Debug, Clone, Serialize)]
pub struct RelationRecord {
    pub relation_id: i64,
    pub from_project: String,
    pub to_project: String,
    pub relation_type: RelationType,
    pub note: Option<String>,
    pub created_at: String,
}

/// One recorded review decision. Reviews are append-only history;
/// the newest row is the current confidence statement.
#[derive(Debug, Clone, Serialize)]
pub struct ReviewRecord {
    pub review_id: i64,
    pub project_id: String,
    pub confidence: Confidence,
    pub note: Option<String>,
    pub reviewed_at: String,
}

/// One portfolio goal.
#[derive(Debug, Clone, Serialize)]
pub struct GoalRecord {
    pub goal_id: i64,
    pub title: String,
    pub status: String,
    pub description: Option<String>,
    pub created_at: String,
    pub projects: Vec<String>,
}

/// The user-owned half of one project's portfolio record.
#[derive(Debug, Clone, Default, Serialize)]
pub struct PortfolioProject {
    pub project_id: String,
    /// `None` until an operator records one; an absent lifecycle
    /// is never reported as `operational` by omission.
    pub lifecycle: Option<Lifecycle>,
    pub confidence: Option<Confidence>,
    pub next_action: Option<String>,
    pub blocker: Option<String>,
    pub reviewed_at: Option<String>,
}

/// One project's full portfolio projection: user-owned fields,
/// the newest snapshot per source, relations in both directions,
/// goal membership and review history.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectPortfolio {
    pub profile: PortfolioProject,
    pub tags: Vec<TagRecord>,
    pub goals: Vec<GoalRecord>,
    /// Outgoing and incoming relations, each labelled with the
    /// direction it was declared in.
    pub relations: Vec<ProjectRelation>,
    pub evidence: Vec<EvidenceSnapshot>,
    pub reviews: Vec<ReviewRecord>,
    pub generated_at: String,
    pub contract: &'static str,
}

/// One relation row projected into a project view.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectRelation {
    /// `outgoing` when this project declared the link,
    /// `incoming` when the other project did.
    pub direction: &'static str,
    pub other_project: String,
    pub relation_type: RelationType,
    pub note: Option<String>,
}

impl ProjectRelation {
    pub fn outgoing(record: &RelationRecord) -> Self {
        ProjectRelation {
            direction: "outgoing",
            other_project: record.to_project.clone(),
            relation_type: record.relation_type,
            note: record.note.clone(),
        }
    }

    pub fn incoming(record: &RelationRecord) -> Self {
        ProjectRelation {
            direction: "incoming",
            other_project: record.from_project.clone(),
            relation_type: record.relation_type,
            note: record.note.clone(),
        }
    }
}

/// The read-model filter combining user-owned and imported fields.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PortfolioFilter {
    pub tag: Option<String>,
    pub lifecycle: Option<Lifecycle>,
    pub confidence: Option<Confidence>,
}

/// One source system and the state its newest snapshot displays.
/// The fleet list carries this instead of the full snapshot so a
/// roster row stays small while still showing that a project's
/// evidence is stale or unavailable.
#[derive(Debug, Clone, Serialize)]
pub struct EvidenceState {
    pub source_system: String,
    pub source_revision: String,
    pub status: EvidenceStatus,
    pub observed_at: String,
}

/// One project row in the portfolio fleet list: the user-owned
/// record, its tags, and the displayed state of every source that
/// has an observation.
#[derive(Debug, Clone, Serialize)]
pub struct PortfolioRow {
    pub profile: PortfolioProject,
    pub tags: Vec<TagRecord>,
    pub evidence: Vec<EvidenceState>,
}

impl PortfolioRow {
    /// One-line evidence summary for a list cell: the source
    /// states joined, or an honest `no evidence` when nothing has
    /// ever been imported. An absent observation never reads as a
    /// pass.
    pub fn evidence_summary(&self) -> String {
        if self.evidence.is_empty() {
            return "no evidence".to_string();
        }
        self.evidence
            .iter()
            .map(|e| format!("{} {}", e.source_system, e.status.label()))
            .collect::<Vec<String>>()
            .join(", ")
    }
}

impl PortfolioFilter {
    pub fn is_empty(&self) -> bool {
        self.tag.is_none() && self.lifecycle.is_none() && self.confidence.is_none()
    }

    /// Whether one project's read model passes the filter. An
    /// absent user-owned field never matches a filter that asks
    /// for a value: an unclassified project is not silently
    /// reported as `incubating`.
    pub fn matches(&self, profile: &PortfolioProject, tags: &[TagRecord]) -> bool {
        if let Some(tag) = &self.tag {
            if !tags.iter().any(|t| &t.name == tag) {
                return false;
            }
        }
        if let Some(lifecycle) = self.lifecycle {
            if profile.lifecycle != Some(lifecycle) {
                return false;
            }
        }
        if let Some(confidence) = self.confidence {
            if profile.confidence != Some(confidence) {
                return false;
            }
        }
        true
    }
}

/// Parse one `key=value` filter parameter. An unknown key or an
/// out-of-vocabulary value is a typed validation error rather than
/// a silently ignored filter, so a mistyped query never widens the
/// result set to everything.
pub fn parse_filter(raw: &str) -> Result<PortfolioFilter, String> {
    let mut filter = PortfolioFilter::default();
    for pair in raw.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let value = value.trim();
        match key.trim() {
            "tag" => filter.tag = Some(validate_tag_name(value)?),
            "lifecycle" => filter.lifecycle = Some(Lifecycle::parse(value)?),
            "confidence" => filter.confidence = Some(Confidence::parse(value)?),
            other => return Err(format!("unknown portfolio filter `{other}`")),
        }
    }
    Ok(filter)
}

/// Render a filter as a query string, omitting unset parts so the
/// portal links carry only what the operator actually chose.
pub fn filter_query(filter: &PortfolioFilter) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(tag) = &filter.tag {
        parts.push(format!("tag={tag}"));
    }
    if let Some(lifecycle) = filter.lifecycle {
        parts.push(format!("lifecycle={}", lifecycle.label()));
    }
    if let Some(confidence) = filter.confidence {
        parts.push(format!("confidence={}", confidence.label()));
    }
    parts.join("&")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(year: i32, month: u32, day: u32) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc
            .with_ymd_and_hms(year, month, day, 0, 0, 0)
            .unwrap()
    }

    #[test]
    fn lifecycle_vocabulary_is_closed_and_round_trips() {
        assert_eq!(
            Lifecycle::labels(),
            vec![
                "incubating",
                "building",
                "validating",
                "operational",
                "paused",
                "archived"
            ]
        );
        for value in Lifecycle::ALL {
            assert_eq!(Lifecycle::parse(value.label()).unwrap(), value);
        }
        assert!(Lifecycle::parse("shipped").is_err());
        assert!(Lifecycle::parse("").is_err());
    }

    #[test]
    fn confidence_vocabulary_is_closed_and_round_trips() {
        assert_eq!(
            Confidence::labels(),
            vec!["unknown", "low", "medium", "high"]
        );
        for value in Confidence::ALL {
            assert_eq!(Confidence::parse(value.label()).unwrap(), value);
        }
        assert!(Confidence::parse("certain").is_err());
    }

    #[test]
    fn relation_vocabulary_is_closed_and_names_the_options() {
        assert_eq!(RelationType::DependsOn.label(), "depends-on");
        assert_eq!(RelationType::OptionalProvider.label(), "optional-provider");
        for value in RelationType::ALL {
            assert_eq!(RelationType::parse(value.label()).unwrap(), value);
        }
        let err = RelationType::parse("blocks").unwrap_err();
        assert!(err.contains("depends-on"), "{err}");
    }

    #[test]
    fn no_relation_type_permits_a_self_link() {
        for value in RelationType::ALL {
            assert!(!value.permits_self_relation(), "{}", value.label());
        }
    }

    #[test]
    fn evidence_status_reports_observation_only_when_observed() {
        assert!(EvidenceStatus::Observed.reports_observation());
        for value in [
            EvidenceStatus::Stale,
            EvidenceStatus::Unavailable,
            EvidenceStatus::Invalid,
            EvidenceStatus::NotRun,
        ] {
            assert!(!value.reports_observation(), "{}", value.label());
        }
    }

    #[test]
    fn evidence_status_parses_every_documented_value() {
        for value in EvidenceStatus::ALL {
            assert_eq!(EvidenceStatus::parse(value.label()).unwrap(), value);
        }
        assert!(EvidenceStatus::parse("healthy").is_err());
        assert!(EvidenceStatus::parse("passed").is_err());
    }

    #[test]
    fn tag_names_are_bounded_kebab_and_never_empty() {
        assert_eq!(validate_tag_name(" platform ").unwrap(), "platform");
        assert_eq!(validate_tag_name("fleet-2").unwrap(), "fleet-2");
        for bad in [
            "",
            "   ",
            "Platform",
            "-platform",
            "platform-",
            "a--b",
            "two words",
        ] {
            assert!(validate_tag_name(bad).is_err(), "{bad}");
        }
        let long = "a".repeat(MAX_TAG_NAME_CHARS + 1);
        assert!(validate_tag_name(&long).is_err());
    }

    #[test]
    fn tag_colors_accept_hex_or_word_and_refuse_anything_else() {
        assert_eq!(validate_tag_color(None).unwrap(), None);
        assert_eq!(validate_tag_color(Some("  ")).unwrap(), None);
        assert_eq!(
            validate_tag_color(Some("#AABBCC")).unwrap(),
            Some("#aabbcc".to_string())
        );
        assert_eq!(
            validate_tag_color(Some("teal")).unwrap(),
            Some("teal".to_string())
        );
        assert!(validate_tag_color(Some("#abc")).is_err());
        assert!(validate_tag_color(Some("red; background:url(x)")).is_err());
    }

    #[test]
    fn notes_are_bounded_and_control_free() {
        assert_eq!(validate_note("blocker", "  ", 10).unwrap(), None);
        assert_eq!(
            validate_note("blocker", " waiting on mac ", 20).unwrap(),
            Some("waiting on mac".to_string())
        );
        assert!(validate_note("blocker", "line\nbreak", 20).is_err());
        assert!(validate_note("blocker", "abcdefghijk", 10).is_err());
    }

    #[test]
    fn source_system_requires_an_honest_bounded_label() {
        assert_eq!(
            validate_source_system(" workspace-governance ").unwrap(),
            "workspace-governance"
        );
        assert_eq!(
            validate_source_system("gate.runtime").unwrap(),
            "gate.runtime"
        );
        for bad in ["", "  ", "-gov", "gov-", "Workspace", "a--b", "gov/x"] {
            assert!(validate_source_system(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn source_revision_is_required_and_control_free() {
        assert_eq!(validate_source_revision(" abc123 ").unwrap(), "abc123");
        assert!(validate_source_revision("").is_err());
        assert!(validate_source_revision("line\nbreak").is_err());
        assert!(validate_source_revision(&"x".repeat(257)).is_err());
    }

    #[test]
    fn goal_status_is_closed() {
        for value in ["planned", "active", "done", "dropped"] {
            assert_eq!(validate_goal_status(value).unwrap(), value);
        }
        assert!(validate_goal_status("blocked").is_err());
    }

    #[test]
    fn prepare_evidence_bounds_parses_and_redacts() {
        let redacted =
            prepare_evidence(r#"{"token":"ghp_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"}"#).unwrap();
        assert!(!redacted.contains("ghp_"), "{redacted}");
        assert!(redacted.contains("[REDACTED]"), "{redacted}");

        assert!(prepare_evidence("not json").is_err());
        assert!(
            prepare_evidence("\"a bare string\"").is_err(),
            "a scalar is not an evidence document"
        );
        assert!(prepare_evidence("[1,2,3]").is_err());
        let oversized = format!("{{\"a\":\"{}\"}}", "x".repeat(MAX_EVIDENCE_BYTES));
        assert!(prepare_evidence(&oversized).is_err());
    }

    #[test]
    fn expiry_downgrades_observed_and_never_upgrades_anything_else() {
        let now = at(2026, 9, 29);
        assert_eq!(
            effective_status(EvidenceStatus::Observed, None, now),
            EvidenceStatus::Observed
        );
        assert_eq!(
            effective_status(EvidenceStatus::Observed, Some("2026-09-30T00:00:00Z"), now),
            EvidenceStatus::Observed
        );
        assert_eq!(
            effective_status(EvidenceStatus::Observed, Some("2026-09-28T00:00:00Z"), now),
            EvidenceStatus::Stale
        );
        for status in [
            EvidenceStatus::Unavailable,
            EvidenceStatus::Invalid,
            EvidenceStatus::NotRun,
            EvidenceStatus::Stale,
        ] {
            assert_eq!(
                effective_status(status, Some("2020-01-01T00:00:00Z"), now),
                status,
                "{}",
                status.label()
            );
        }
        // An unparseable bound is not a freshness claim.
        assert_eq!(
            effective_status(EvidenceStatus::Observed, Some("not-a-date"), now),
            EvidenceStatus::Observed
        );
    }

    #[test]
    fn filter_matches_only_declared_values() {
        let profile = PortfolioProject {
            project_id: "alethefy".to_string(),
            lifecycle: Some(Lifecycle::Building),
            confidence: Some(Confidence::High),
            ..PortfolioProject::default()
        };
        let tags = vec![TagRecord {
            tag_id: 1,
            name: "platform".to_string(),
            color: None,
            created_at: "2026-09-29T00:00:00Z".to_string(),
        }];
        assert!(PortfolioFilter {
            tag: Some("platform".to_string()),
            lifecycle: Some(Lifecycle::Building),
            confidence: Some(Confidence::High),
        }
        .matches(&profile, &tags));
        assert!(!PortfolioFilter {
            tag: Some("platform".to_string()),
            lifecycle: Some(Lifecycle::Operational),
            confidence: None,
        }
        .matches(&profile, &tags));
        assert!(!PortfolioFilter {
            tag: Some("frontend".to_string()),
            ..PortfolioFilter::default()
        }
        .matches(&profile, &tags));

        // An unclassified project never matches a declared filter.
        let bare = PortfolioProject {
            project_id: "new".to_string(),
            ..PortfolioProject::default()
        };
        assert!(!PortfolioFilter {
            lifecycle: Some(Lifecycle::Incubating),
            ..PortfolioFilter::default()
        }
        .matches(&bare, &[]));
        assert!(PortfolioFilter::default().matches(&bare, &[]));
    }

    #[test]
    fn parse_filter_reads_every_documented_key_and_refuses_others() {
        let filter = parse_filter("tag=platform&lifecycle=building&confidence=high").unwrap();
        assert_eq!(filter.tag.as_deref(), Some("platform"));
        assert_eq!(filter.lifecycle, Some(Lifecycle::Building));
        assert_eq!(filter.confidence, Some(Confidence::High));
        assert!(PortfolioFilter::default().is_empty());
        assert!(!filter.is_empty());
        assert_eq!(
            filter_query(&filter),
            "tag=platform&lifecycle=building&confidence=high"
        );
        assert_eq!(filter_query(&PortfolioFilter::default()), "");

        assert!(parse_filter("stage=alpha").is_err());
        assert!(parse_filter("lifecycle=shipped").is_err());
        assert!(parse_filter("tag=Platform").is_err());
        assert!(parse_filter("tag=platform&lifecycle=building").is_ok());
    }

    #[test]
    fn project_relation_projection_labels_direction() {
        let record = RelationRecord {
            relation_id: 1,
            from_project: "alethefy".to_string(),
            to_project: "forge".to_string(),
            relation_type: RelationType::DependsOn,
            note: None,
            created_at: "2026-09-29T00:00:00Z".to_string(),
        };
        let out = ProjectRelation::outgoing(&record);
        assert_eq!(out.direction, "outgoing");
        assert_eq!(out.other_project, "forge");
        let inc = ProjectRelation::incoming(&record);
        assert_eq!(inc.direction, "incoming");
        assert_eq!(inc.other_project, "alethefy");
    }
}
