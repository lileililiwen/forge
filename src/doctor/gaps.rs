//! Evidence-backed project findings
//! (`project-evidence-gap-assessment`).
//!
//! The gaps package is a **projection** over the catalog contract: it
//! consumes [`CatalogRecord`]s the catalog sources have already produced
//! and emits one typed finding per (category, subject) per record. A
//! finding is an observation, never a stored verdict promoted to
//! health, and the package is strictly read-only: it does not write any
//! file, registry byte, table row, journal row, provider value or CI
//! execution. Repair is the responsibility of a separate package
//! (`project-local-remediation-plans`).
//!
//! ## Vocabularies
//!
//! - [`GapStatus`] is the closed set of verdicts: `pass`, `warn`, `fail`,
//!   `unavailable`, `not_applicable`. An unavailable source is never
//!   reported as healthy by silence; a not-applicable control is
//!   excluded from any healthy-or-failed count.
//! - [`GapCategory`] is the closed set of categories: `description`,
//!   `tags`, `ci`, `compose`, `manifest`, `docs`, `repository`.
//! - [`RemediationClass`] is the spec's `automatic | semantic | manual`
//!   vocabulary; the existing [`crate::doctor::Remediation`] enum is
//!   reused for serialization (`Automatic → automatic`, `Ai → semantic`,
//!   `Manual → manual`).
//!
//! ## Finding id
//!
//! The finding id is stable and derived from `(category, project_id,
//! subject)`, in the form `gaps.<category>.<project_id>.<subject>`. A
//! duplicate `project_id` across two selected sources is rendered twice
//! (the catalog contract never merges records silently), and the source
//! label travels inside the finding's evidence so a downstream consumer
//! can attribute it.

use serde::{Deserialize, Serialize};

use crate::catalog::record::{CatalogRecord, EvidenceState, SourceKind};
use crate::doctor::Remediation;

/// Versioned contract for the project evidence-gap surface.
pub const GAPS_CONTRACT_VERSION: &str = "forge-project-evidence/0.1.0";

/// Closed set of gap categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GapCategory {
    Description,
    Tags,
    Ci,
    Compose,
    Manifest,
    Docs,
    Repository,
}

impl GapCategory {
    /// Stable kebab-case id.
    pub fn id(self) -> &'static str {
        match self {
            GapCategory::Description => "description",
            GapCategory::Tags => "tags",
            GapCategory::Ci => "ci",
            GapCategory::Compose => "compose",
            GapCategory::Manifest => "manifest",
            GapCategory::Docs => "docs",
            GapCategory::Repository => "repository",
        }
    }

    /// Parse a `--category` selector. Unknown values are refused.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "description" => Some(GapCategory::Description),
            "tags" => Some(GapCategory::Tags),
            "ci" => Some(GapCategory::Ci),
            "compose" => Some(GapCategory::Compose),
            "manifest" => Some(GapCategory::Manifest),
            "docs" => Some(GapCategory::Docs),
            "repository" => Some(GapCategory::Repository),
            _ => None,
        }
    }

    /// Every category, in the order findings are emitted.
    pub fn all() -> [GapCategory; 7] {
        [
            GapCategory::Description,
            GapCategory::Tags,
            GapCategory::Ci,
            GapCategory::Compose,
            GapCategory::Manifest,
            GapCategory::Docs,
            GapCategory::Repository,
        ]
    }
}

impl std::fmt::Display for GapCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id())
    }
}

/// Closed set of gap verdicts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GapStatus {
    /// The gap is filled by the available evidence.
    #[serde(rename = "pass")]
    Pass,
    /// The value is present but stale, partial or otherwise non-current.
    #[serde(rename = "warn")]
    Warn,
    /// The gap is open: the value is missing or invalid.
    #[serde(rename = "fail")]
    Fail,
    /// The source cannot be read.
    #[serde(rename = "unavailable")]
    Unavailable,
    /// The control does not apply to the record's source or profile.
    #[serde(rename = "not_applicable")]
    NotApplicable,
}

impl GapStatus {
    /// Stable kebab-case id.
    pub fn id(self) -> &'static str {
        match self {
            GapStatus::Pass => "pass",
            GapStatus::Warn => "warn",
            GapStatus::Fail => "fail",
            GapStatus::Unavailable => "unavailable",
            GapStatus::NotApplicable => "not_applicable",
        }
    }

    /// Parse a `--status` selector. Unknown values are refused.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "pass" => Some(GapStatus::Pass),
            "warn" => Some(GapStatus::Warn),
            "fail" => Some(GapStatus::Fail),
            "unavailable" => Some(GapStatus::Unavailable),
            "not_applicable" | "not-applicable" | "na" => Some(GapStatus::NotApplicable),
            _ => None,
        }
    }

    /// Whether this verdict denies health.
    pub fn blocks_health(self) -> bool {
        matches!(self, GapStatus::Fail | GapStatus::Unavailable)
    }
}

impl std::fmt::Display for GapStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id())
    }
}

/// Spec's `automatic | semantic | manual` vocabulary. Reused rather
/// than invented to share the doctor and spec-routing classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RemediationClass {
    /// Deterministic generator or script produces the value.
    Automatic,
    /// An AI/LLM workflow generates the value.
    Semantic,
    /// A human owner produces the value.
    Manual,
}

impl RemediationClass {
    /// Stable kebab-case id.
    pub fn id(self) -> &'static str {
        match self {
            RemediationClass::Automatic => "automatic",
            RemediationClass::Semantic => "semantic",
            RemediationClass::Manual => "manual",
        }
    }

    /// Parse a `--remediation-class` selector. Unknown values are refused.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "automatic" => Some(RemediationClass::Automatic),
            "semantic" | "ai" => Some(RemediationClass::Semantic),
            "manual" => Some(RemediationClass::Manual),
            _ => None,
        }
    }

    /// Map into the existing doctor [`Remediation`] enum so a spec router
    /// can consume the finding without a second classification table.
    pub fn as_doctor(self) -> Remediation {
        match self {
            RemediationClass::Automatic => Remediation::Automatic,
            RemediationClass::Semantic => Remediation::Ai,
            RemediationClass::Manual => Remediation::Manual,
        }
    }
}

impl std::fmt::Display for RemediationClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id())
    }
}

/// One typed evidence-gap finding for one record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GapFinding {
    /// Stable id derived from `(category, project_id, subject)`.
    pub id: String,
    /// Verdict.
    pub status: GapStatus,
    /// Closed category.
    pub category: GapCategory,
    /// Subject within the category (e.g. `name`, `tags`, `ci`).
    pub subject: String,
    /// Remediation class (spec vocabulary).
    pub remediation_class: RemediationClass,
    /// Project id the finding is attributed to.
    pub project_id: String,
    /// Source label the record came from (`local`, `workspace-registry`,
    /// `inventory`, `git:<path>`).
    pub source: String,
    /// Source kind tag from the catalog record.
    pub source_kind: SourceKind,
    /// Source revision (the catalog's `source_revision`), if present.
    pub source_revision: Option<String>,
    /// Observation timestamp (the catalog's `observed_at`).
    pub observed_at: String,
    /// Whether the underlying record was `fresh` or `stale` at read time.
    pub freshness: crate::catalog::record::Freshness,
    /// Evidence backing the verdict. Redacted via
    /// `policy::redact_credentials`; a credential-shaped value never
    /// appears in the rendered finding.
    pub evidence: Vec<String>,
    /// Human-facing detail (one short clause; never echoes a secret).
    pub detail: String,
}

/// Full evidence-gap report over a slice of records. Ordering is stable
/// by `(project_id, source, category, subject)` so two readers of the
/// same catalog produce identical bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GapReport {
    pub contract: String,
    pub project_id: Option<String>,
    pub findings: Vec<GapFinding>,
}

impl GapReport {
    /// True when every applicable (non-`not_applicable`) finding passes.
    pub fn clean(&self) -> bool {
        self.findings
            .iter()
            .all(|f| matches!(f.status, GapStatus::Pass | GapStatus::NotApplicable))
    }
}

/// Compose the closed category, status and remediation-class filters
/// used by the CLI. An empty filter accepts everything.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GapFilters {
    pub categories: Vec<GapCategory>,
    pub statuses: Vec<GapStatus>,
    pub remediation_classes: Vec<RemediationClass>,
}

impl GapFilters {
    /// True when the finding passes every active filter.
    pub fn matches(&self, finding: &GapFinding) -> bool {
        if !self.categories.is_empty() && !self.categories.contains(&finding.category) {
            return false;
        }
        if !self.statuses.is_empty() && !self.statuses.contains(&finding.status) {
            return false;
        }
        if !self.remediation_classes.is_empty()
            && !self
                .remediation_classes
                .contains(&finding.remediation_class)
        {
            return false;
        }
        true
    }
}

/// Compute the gap findings for a single record. One finding per
/// category in the order [`GapCategory::all`] returns, except
/// `not_applicable` findings are still emitted so a consumer can count
/// the inapplicable controls explicitly.
pub fn findings_for_record(record: &CatalogRecord) -> Vec<GapFinding> {
    let mut out = Vec::with_capacity(GapCategory::all().len());
    for category in GapCategory::all() {
        out.push(evaluate(record, category));
    }
    out
}

/// Build the full gap report over the given records. When `project_id`
/// is `Some`, the report is single-project scoped and emits only
/// records for that id (the caller should pre-filter); the field stays
/// in the report envelope so a consumer can scope its queries.
///
/// Unavailable sources (those that returned a `SourceStatus` with
/// `state == "unavailable"`) are surfaced as one `unavailable` finding
/// per closed category, named by the source, so a failed read is never
/// reported as a silent zero. The `github` source is a permanent
/// "not implemented" state and is surfaced through the source status
/// only — it never generates findings.
pub fn build_report(
    records: &[CatalogRecord],
    source_statuses: &[crate::catalog::SourceStatus],
    project_id: Option<&str>,
) -> GapReport {
    let mut findings: Vec<GapFinding> = records.iter().flat_map(findings_for_record).collect();
    for status in source_statuses {
        if status.state == "unavailable" && status.source_kind != SourceKind::Github {
            for category in GapCategory::all() {
                findings.push(synthesize_unavailable_finding(status, category));
            }
        }
    }
    findings.sort_by(|a, b| {
        a.project_id
            .cmp(&b.project_id)
            .then_with(|| a.source.cmp(&b.source))
            .then_with(|| a.category.id().cmp(b.category.id()))
            .then_with(|| a.subject.cmp(&b.subject))
    });
    GapReport {
        contract: GAPS_CONTRACT_VERSION.to_string(),
        project_id: project_id.map(|s| s.to_string()),
        findings,
    }
}

/// One synthesized `unavailable` finding for a source that could not be
/// read. The source string is used as the `project_id` surrogate (it is
/// the best identifier we have when no records were returned) so the
/// finding has a stable, unique id of the form
/// `gaps.<category>.<source>.<subject>`.
fn synthesize_unavailable_finding(
    status: &crate::catalog::SourceStatus,
    category: GapCategory,
) -> GapFinding {
    let reason = status
        .reason
        .clone()
        .unwrap_or_else(|| "source is unavailable".to_string());
    let subject = category.id();
    let id = format!("gaps.{}.{}.{}", category.id(), status.source, subject);
    let detail = format!("source is unavailable: {reason}");
    let evidence = vec![reason.clone()];
    GapFinding {
        id,
        status: GapStatus::Unavailable,
        category,
        subject: subject.to_string(),
        remediation_class: RemediationClass::Manual,
        project_id: status.source.clone(),
        source: status.source.clone(),
        source_kind: status.source_kind,
        source_revision: None,
        observed_at: String::new(),
        freshness: crate::catalog::record::Freshness::Unknown,
        evidence,
        detail,
    }
}

/// Internal helper: extract the (subject, status, remediation, evidence,
/// detail) tuple from a rule.
type RuleResult = (
    &'static str,
    GapStatus,
    RemediationClass,
    Vec<String>,
    String,
);

/// Render the gap report as a human-facing table. The table is not the
/// machine contract (JSON and NDJSON are); the layout exists so a
/// operator can scan findings without parsing JSON.
pub fn render_report_human(
    report: &GapReport,
    filtered: &[&GapFinding],
    sources: &[crate::catalog::SourceStatus],
    total: usize,
    filters: &GapFilters,
) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "forge project gaps — contract {}", report.contract);
    if let Some(id) = report.project_id.as_deref() {
        let _ = writeln!(out, "project: {id}");
    } else {
        let _ = writeln!(out, "project: (catalog)");
    }
    if sources.is_empty() {
        let _ = writeln!(out, "sources: (none)");
    } else {
        for status in sources {
            let _ = writeln!(
                out,
                "source: {} [{}] state={} records={}{}",
                status.source,
                status.source_kind,
                status.state,
                status.records,
                status
                    .reason
                    .as_deref()
                    .map(|r| format!(" reason={r}"))
                    .unwrap_or_default()
            );
        }
    }
    let _ = writeln!(
        out,
        "findings: {} of {} ({} categories, {} statuses, {} remediation-classes)",
        filtered.len(),
        total,
        filters.categories.len(),
        filters.statuses.len(),
        filters.remediation_classes.len(),
    );
    if filtered.is_empty() {
        let _ = writeln!(out, "(no findings)");
        return out;
    }
    #[allow(clippy::write_literal)]
    let _ = writeln!(
        out,
        "{:<44} {:<18} {:<14} {:<11} {:<10} {}",
        "ID", "CATEGORY", "SUBJECT", "STATUS", "REMED.", "DETAIL",
    );
    for finding in filtered {
        let _ = writeln!(
            out,
            "{:<44} {:<18} {:<14} {:<11} {:<10} {}",
            truncate(&finding.id, 44),
            finding.category.id(),
            truncate(&finding.subject, 14),
            finding.status.id(),
            finding.remediation_class.id(),
            truncate(&finding.detail, 80),
        );
    }
    out
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let kept: String = value.chars().take(max.saturating_sub(1)).collect();
    format!("{kept}…")
}

fn evaluate(record: &CatalogRecord, category: GapCategory) -> GapFinding {
    let (subject, status, remediation, evidence, detail) = match category {
        GapCategory::Description => description_rule(record),
        GapCategory::Tags => tags_rule(record),
        GapCategory::Ci => ci_rule(record),
        GapCategory::Compose => compose_rule(record),
        GapCategory::Manifest => manifest_rule(record),
        GapCategory::Docs => docs_rule(record),
        GapCategory::Repository => repository_rule(record),
    };
    let detail = crate::policy::redact_credentials(&detail);
    let evidence = evidence
        .into_iter()
        .map(|line| crate::policy::redact_credentials(&line))
        .collect::<Vec<_>>();
    let id = format!("gaps.{}.{}.{}", category.id(), record.project_id, subject);
    GapFinding {
        id,
        status,
        category,
        subject: subject.to_string(),
        remediation_class: remediation,
        project_id: record.project_id.clone(),
        source: record.source.clone(),
        source_kind: record.source_kind,
        source_revision: record.source_revision.clone(),
        observed_at: record.observed_at.clone(),
        freshness: record.freshness,
        evidence,
        detail,
    }
}

fn description_rule(record: &CatalogRecord) -> RuleResult {
    // Description only fills from the local registry; the workspace
    // and inventory sources do not carry a human name.
    if record.source_kind == SourceKind::Github {
        return (
            "name",
            GapStatus::NotApplicable,
            RemediationClass::Manual,
            vec!["github metadata adapter does not record a project name".to_string()],
            "name is not a github metadata field".to_string(),
        );
    }
    if record.source_kind != SourceKind::Local {
        return (
            "name",
            GapStatus::NotApplicable,
            RemediationClass::Manual,
            vec![format!(
                "{} sources do not carry a project name",
                record.source_kind
            )],
            "name is not applicable to this source".to_string(),
        );
    }
    match record.evidence {
        EvidenceState::Present => {
            if record.name.as_deref().is_some_and(|n| !n.is_empty()) {
                (
                    "name",
                    GapStatus::Pass,
                    RemediationClass::Manual,
                    vec![format!(
                        "name={}",
                        record.name.as_deref().unwrap_or("unknown")
                    )],
                    "project name is recorded in the local registry".to_string(),
                )
            } else {
                (
                    "name",
                    GapStatus::Fail,
                    RemediationClass::Manual,
                    vec!["name is absent from the local registry record".to_string()],
                    "project name is missing from the local registry".to_string(),
                )
            }
        }
        EvidenceState::Unavailable => (
            "name",
            GapStatus::Unavailable,
            RemediationClass::Manual,
            vec!["local registry is unreadable".to_string()],
            "local registry could not be read".to_string(),
        ),
        EvidenceState::Unverified => (
            "name",
            GapStatus::Warn,
            RemediationClass::Manual,
            vec!["local registry is present but unverified".to_string()],
            "local registry record is unverified".to_string(),
        ),
        EvidenceState::Stale => (
            "name",
            GapStatus::Warn,
            RemediationClass::Manual,
            vec!["local registry observation is stale".to_string()],
            "local registry record is stale".to_string(),
        ),
        EvidenceState::Absent => (
            "name",
            GapStatus::Fail,
            RemediationClass::Manual,
            vec!["local registry has no project row".to_string()],
            "local registry has no project row for this id".to_string(),
        ),
    }
}

fn tags_rule(record: &CatalogRecord) -> RuleResult {
    if record.evidence == EvidenceState::Unavailable {
        return (
            "tags",
            GapStatus::Unavailable,
            RemediationClass::Manual,
            vec!["source could not be read".to_string()],
            "tags source is unreadable".to_string(),
        );
    }
    if record.freshness == crate::catalog::record::Freshness::Stale {
        return (
            "tags",
            GapStatus::Warn,
            RemediationClass::Manual,
            vec!["tags observation is stale".to_string()],
            "tags observation is stale".to_string(),
        );
    }
    if record.tags.is_empty() {
        return (
            "tags",
            GapStatus::Fail,
            RemediationClass::Manual,
            vec!["no tags recorded on the project record".to_string()],
            "project has no tags".to_string(),
        );
    }
    (
        "tags",
        GapStatus::Pass,
        RemediationClass::Manual,
        vec![format!("{} tag(s)", record.tags.len())],
        "project tags are recorded".to_string(),
    )
}

fn ci_rule(record: &CatalogRecord) -> RuleResult {
    if record.evidence == EvidenceState::Unavailable {
        return (
            "ci",
            GapStatus::Unavailable,
            RemediationClass::Manual,
            vec!["source could not be read".to_string()],
            "ci source is unreadable".to_string(),
        );
    }
    let present = record.ci.as_deref().is_some_and(|v| !v.is_empty());
    if record.freshness == crate::catalog::record::Freshness::Stale {
        if present {
            return (
                "ci",
                GapStatus::Warn,
                RemediationClass::Automatic,
                vec![format!("ci={} (stale)", record.ci.as_deref().unwrap())],
                "ci observation is stale".to_string(),
            );
        }
        return (
            "ci",
            GapStatus::Warn,
            RemediationClass::Automatic,
            vec!["ci observation is stale and the value is absent".to_string()],
            "ci observation is stale and the value is absent".to_string(),
        );
    }
    if present {
        return (
            "ci",
            GapStatus::Pass,
            RemediationClass::Automatic,
            vec![format!("ci={}", record.ci.as_deref().unwrap())],
            "ci evidence is recorded".to_string(),
        );
    }
    (
        "ci",
        GapStatus::Fail,
        RemediationClass::Automatic,
        vec!["no ci state recorded on the project record".to_string()],
        "project has no ci evidence".to_string(),
    )
}

fn compose_rule(record: &CatalogRecord) -> RuleResult {
    if record.evidence == EvidenceState::Unavailable {
        return (
            "compose",
            GapStatus::Unavailable,
            RemediationClass::Manual,
            vec!["source could not be read".to_string()],
            "compose source is unreadable".to_string(),
        );
    }
    let present = record.compose.as_deref().is_some_and(|v| !v.is_empty());
    if record.freshness == crate::catalog::record::Freshness::Stale {
        if present {
            return (
                "compose",
                GapStatus::Warn,
                RemediationClass::Manual,
                vec![format!(
                    "compose={} (stale)",
                    record.compose.as_deref().unwrap()
                )],
                "compose observation is stale".to_string(),
            );
        }
        return (
            "compose",
            GapStatus::Warn,
            RemediationClass::Manual,
            vec!["compose observation is stale and the value is absent".to_string()],
            "compose observation is stale and the value is absent".to_string(),
        );
    }
    if present {
        return (
            "compose",
            GapStatus::Pass,
            RemediationClass::Manual,
            vec![format!("compose={}", record.compose.as_deref().unwrap())],
            "compose evidence is recorded".to_string(),
        );
    }
    (
        "compose",
        GapStatus::Fail,
        RemediationClass::Manual,
        vec!["no compose state recorded on the project record".to_string()],
        "project has no compose evidence".to_string(),
    )
}

fn manifest_rule(record: &CatalogRecord) -> RuleResult {
    match record.evidence {
        EvidenceState::Present => match record.freshness {
            crate::catalog::record::Freshness::Current => (
                "manifest",
                GapStatus::Pass,
                RemediationClass::Manual,
                vec![format!("observed_at={}", record.observed_at)],
                "manifest is recorded with a current observation".to_string(),
            ),
            crate::catalog::record::Freshness::Stale => (
                "manifest",
                GapStatus::Warn,
                RemediationClass::Manual,
                vec![format!("observed_at={} (stale)", record.observed_at)],
                "manifest is recorded but the observation is stale".to_string(),
            ),
            crate::catalog::record::Freshness::Unknown => (
                "manifest",
                GapStatus::Warn,
                RemediationClass::Manual,
                vec![format!("observed_at={} (unparseable)", record.observed_at)],
                "manifest observation timestamp could not be parsed".to_string(),
            ),
        },
        EvidenceState::Absent => (
            "manifest",
            GapStatus::Fail,
            RemediationClass::Manual,
            vec!["no project record exists in the source".to_string()],
            "no project record exists in the source".to_string(),
        ),
        EvidenceState::Stale => (
            "manifest",
            GapStatus::Warn,
            RemediationClass::Manual,
            vec![format!("observed_at={} (stale)", record.observed_at)],
            "manifest is stale and never current".to_string(),
        ),
        EvidenceState::Unavailable => (
            "manifest",
            GapStatus::Unavailable,
            RemediationClass::Manual,
            vec!["source could not be read".to_string()],
            "manifest source is unreadable".to_string(),
        ),
        EvidenceState::Unverified => (
            "manifest",
            GapStatus::Warn,
            RemediationClass::Manual,
            vec!["manifest value is present but unverified".to_string()],
            "manifest value is present but unverified".to_string(),
        ),
    }
}

fn docs_rule(record: &CatalogRecord) -> RuleResult {
    // The catalog record carries no docs field; the only sources that
    // could answer a docs question are local and git, and even then
    // only the on-disk `forge doctor` inspection knows the answer.
    // Stay explicit: the gap is never silently omitted and never
    // inferred from a side channel.
    if matches!(
        record.source_kind,
        SourceKind::Github | SourceKind::WorkspaceRegistry | SourceKind::Inventory
    ) {
        return (
            "docs",
            GapStatus::NotApplicable,
            RemediationClass::Manual,
            vec![format!(
                "{} sources do not expose a docs field",
                record.source_kind
            )],
            "docs is not a field on this source".to_string(),
        );
    }
    (
        "docs",
        GapStatus::NotApplicable,
        RemediationClass::Manual,
        vec!["docs evidence is collected by `forge doctor` not the catalog".to_string()],
        "docs evidence is collected by `forge doctor`, not the catalog; not applicable here"
            .to_string(),
    )
}

fn repository_rule(record: &CatalogRecord) -> RuleResult {
    if record.evidence == EvidenceState::Unavailable {
        return (
            "repository",
            GapStatus::Unavailable,
            RemediationClass::Manual,
            vec!["source could not be read".to_string()],
            "repository source is unreadable".to_string(),
        );
    }
    let present = record.repository.as_deref().is_some_and(|v| !v.is_empty());
    if record.freshness == crate::catalog::record::Freshness::Stale {
        if present {
            return (
                "repository",
                GapStatus::Warn,
                RemediationClass::Manual,
                vec![format!(
                    "repository={} (stale)",
                    record.repository.as_deref().unwrap()
                )],
                "repository observation is stale".to_string(),
            );
        }
        return (
            "repository",
            GapStatus::Warn,
            RemediationClass::Manual,
            vec!["repository observation is stale and the value is absent".to_string()],
            "repository observation is stale and the value is absent".to_string(),
        );
    }
    if present {
        return (
            "repository",
            GapStatus::Pass,
            RemediationClass::Manual,
            vec![format!(
                "repository={}",
                record.repository.as_deref().unwrap()
            )],
            "repository evidence is recorded".to_string(),
        );
    }
    (
        "repository",
        GapStatus::Fail,
        RemediationClass::Manual,
        vec!["no repository recorded on the project record".to_string()],
        "project has no repository evidence".to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::record::{EvidenceState, Freshness, SourceKind};

    #[allow(clippy::too_many_arguments)]
    fn record(
        project_id: &str,
        source: &str,
        kind: SourceKind,
        evidence: EvidenceState,
        name: Option<&str>,
        tags: &[&str],
        ci: Option<&str>,
        compose: Option<&str>,
        repository: Option<&str>,
    ) -> CatalogRecord {
        CatalogRecord {
            project_id: project_id.to_string(),
            name: name.map(|n| n.to_string()),
            source: source.to_string(),
            source_kind: kind,
            source_revision: Some("0123456789abcdef0123456789abcdef01234567".to_string()),
            observed_at: "2026-09-29T00:00:00Z".to_string(),
            freshness: Freshness::Current,
            profile: Some("rust-web".to_string()),
            lifecycle: Some("operational".to_string()),
            repository: repository.map(|r| r.to_string()),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            languages: vec!["rust".to_string()],
            ci: ci.map(|c| c.to_string()),
            compose: compose.map(|c| c.to_string()),
            evidence,
        }
    }

    #[test]
    fn vocabularies_parse_their_known_values() {
        for category in GapCategory::all() {
            assert_eq!(GapCategory::parse(category.id()), Some(category));
        }
        assert_eq!(GapCategory::parse("nope"), None);
        for status in [
            GapStatus::Pass,
            GapStatus::Warn,
            GapStatus::Fail,
            GapStatus::Unavailable,
            GapStatus::NotApplicable,
        ] {
            assert_eq!(GapStatus::parse(status.id()), Some(status));
        }
        assert_eq!(GapStatus::parse("bogus"), None);
        for class in [
            RemediationClass::Automatic,
            RemediationClass::Semantic,
            RemediationClass::Manual,
        ] {
            assert_eq!(RemediationClass::parse(class.id()), Some(class));
        }
        assert_eq!(RemediationClass::parse("nope"), None);
    }

    #[test]
    fn stable_id_includes_category_project_and_subject() {
        let rec = record(
            "alpha",
            "local",
            SourceKind::Local,
            EvidenceState::Present,
            Some("Alpha"),
            &["product"],
            Some("passing"),
            Some("compose_ready"),
            Some("https://example.invalid/alpha.git"),
        );
        let findings = findings_for_record(&rec);
        let description = findings
            .iter()
            .find(|f| f.category == GapCategory::Description)
            .unwrap();
        assert_eq!(description.id, "gaps.description.alpha.name");
        assert_eq!(description.status, GapStatus::Pass);
    }

    #[test]
    fn missing_name_in_local_is_a_distinct_finding() {
        let rec = record(
            "alpha",
            "local",
            SourceKind::Local,
            EvidenceState::Present,
            None,
            &["product"],
            Some("passing"),
            Some("compose_ready"),
            Some("https://example.invalid/alpha.git"),
        );
        let findings = findings_for_record(&rec);
        let description = findings
            .iter()
            .find(|f| f.category == GapCategory::Description)
            .unwrap();
        let tags = findings
            .iter()
            .find(|f| f.category == GapCategory::Tags)
            .unwrap();
        assert_eq!(description.status, GapStatus::Fail);
        assert_eq!(tags.status, GapStatus::Pass);
    }

    #[test]
    fn empty_tags_yield_a_finding_separate_from_description() {
        let rec = record(
            "alpha",
            "local",
            SourceKind::Local,
            EvidenceState::Present,
            Some("Alpha"),
            &[],
            Some("passing"),
            Some("compose_ready"),
            Some("https://example.invalid/alpha.git"),
        );
        let findings = findings_for_record(&rec);
        let tags = findings
            .iter()
            .find(|f| f.category == GapCategory::Tags)
            .unwrap();
        let description = findings
            .iter()
            .find(|f| f.category == GapCategory::Description)
            .unwrap();
        assert_eq!(tags.status, GapStatus::Fail);
        assert_eq!(description.status, GapStatus::Pass);
        assert_ne!(tags.id, description.id);
    }

    #[test]
    fn unavailable_evidence_is_unavailable_not_fail() {
        let rec = record(
            "alpha",
            "workspace-registry",
            SourceKind::WorkspaceRegistry,
            EvidenceState::Unavailable,
            None,
            &[],
            None,
            None,
            None,
        );
        let findings = findings_for_record(&rec);
        for category in [
            GapCategory::Tags,
            GapCategory::Ci,
            GapCategory::Compose,
            GapCategory::Repository,
        ] {
            let f = findings.iter().find(|f| f.category == category).unwrap();
            assert_eq!(f.status, GapStatus::Unavailable, "{category:?}");
        }
    }

    #[test]
    fn stale_observation_warns_when_value_present() {
        let mut rec = record(
            "alpha",
            "inventory",
            SourceKind::Inventory,
            EvidenceState::Stale,
            None,
            &["platform"],
            Some("passing"),
            Some("compose_ready"),
            Some("https://example.invalid/alpha.git"),
        );
        rec.freshness = crate::catalog::record::Freshness::Stale;
        let findings = findings_for_record(&rec);
        let ci = findings
            .iter()
            .find(|f| f.category == GapCategory::Ci)
            .unwrap();
        let compose = findings
            .iter()
            .find(|f| f.category == GapCategory::Compose)
            .unwrap();
        assert_eq!(ci.status, GapStatus::Warn);
        assert_eq!(compose.status, GapStatus::Warn);
    }

    #[test]
    fn github_source_marks_description_not_applicable() {
        let rec = record(
            "alpha",
            "github",
            SourceKind::Github,
            EvidenceState::Present,
            None,
            &["platform"],
            None,
            None,
            Some("https://example.invalid/alpha.git"),
        );
        let findings = findings_for_record(&rec);
        let description = findings
            .iter()
            .find(|f| f.category == GapCategory::Description)
            .unwrap();
        assert_eq!(description.status, GapStatus::NotApplicable);
    }

    #[test]
    fn docs_is_not_applicable_for_every_source() {
        let rec = record(
            "alpha",
            "local",
            SourceKind::Local,
            EvidenceState::Present,
            Some("Alpha"),
            &["product"],
            Some("passing"),
            Some("compose_ready"),
            Some("https://example.invalid/alpha.git"),
        );
        let findings = findings_for_record(&rec);
        let docs = findings
            .iter()
            .find(|f| f.category == GapCategory::Docs)
            .unwrap();
        assert_eq!(docs.status, GapStatus::NotApplicable);
    }

    #[test]
    fn filters_compose_with_and_semantics() {
        let rec = record(
            "alpha",
            "local",
            SourceKind::Local,
            EvidenceState::Present,
            Some("Alpha"),
            &[],
            None,
            None,
            Some("https://example.invalid/alpha.git"),
        );
        let findings = findings_for_record(&rec);
        let f = GapFilters {
            categories: vec![GapCategory::Tags],
            statuses: vec![GapStatus::Fail],
            remediation_classes: vec![],
        };
        let tags = findings
            .iter()
            .find(|f| f.category == GapCategory::Tags)
            .unwrap();
        let description = findings
            .iter()
            .find(|f| f.category == GapCategory::Description)
            .unwrap();
        assert!(f.matches(tags));
        assert!(!f.matches(description));
    }

    #[test]
    fn report_orders_findings_by_project_then_source_then_category_then_subject() {
        let rec1 = record(
            "beta",
            "workspace-registry",
            SourceKind::WorkspaceRegistry,
            EvidenceState::Present,
            None,
            &["platform"],
            Some("passing"),
            None,
            Some("https://example.invalid/beta.git"),
        );
        let rec2 = record(
            "alpha",
            "local",
            SourceKind::Local,
            EvidenceState::Present,
            Some("Alpha"),
            &["product"],
            Some("passing"),
            Some("compose_ready"),
            Some("https://example.invalid/alpha.git"),
        );
        let report = build_report(&[rec1, rec2], &[], None);
        let mut last: Option<(&str, &str, &str, &str)> = None;
        for f in &report.findings {
            let key = (
                f.project_id.as_str(),
                f.source.as_str(),
                f.category.id(),
                f.subject.as_str(),
            );
            if let Some(prev) = last.as_ref() {
                assert!(prev <= &key, "{prev:?} > {key:?}");
            }
            last = Some(key);
        }
    }

    #[test]
    fn finding_evidence_is_redacted() {
        let rec = record(
            "alpha",
            "local",
            SourceKind::Local,
            EvidenceState::Present,
            Some("Alpha"),
            &["product"],
            Some("passing"),
            Some("compose_ready"),
            Some("https://user:ghp_abcdefghijklmnopqrstuvwxyz0123456789@x/y.git"),
        );
        let findings = findings_for_record(&rec);
        for finding in &findings {
            for line in &finding.evidence {
                assert!(!line.contains("ghp_abcdef"), "{line}");
            }
            assert!(!finding.detail.contains("ghp_abcdef"), "{}", finding.detail);
        }
    }

    #[test]
    fn remediation_class_maps_to_doctor_remediation() {
        assert_eq!(
            RemediationClass::Automatic.as_doctor(),
            Remediation::Automatic
        );
        assert_eq!(RemediationClass::Semantic.as_doctor(), Remediation::Ai);
        assert_eq!(RemediationClass::Manual.as_doctor(), Remediation::Manual);
    }

    #[test]
    fn clean_report_has_no_fail_or_unavailable() {
        let rec = record(
            "alpha",
            "local",
            SourceKind::Local,
            EvidenceState::Present,
            Some("Alpha"),
            &["product"],
            Some("passing"),
            Some("compose_ready"),
            Some("https://example.invalid/alpha.git"),
        );
        let report = build_report(std::slice::from_ref(&rec), &[], None);
        assert!(report.clean());
    }
}
