//! Portfolio interest domain: aggregate, privacy-safe evidence about
//! which projects deserve deeper investment.
//!
//! This package is the *only* way interest evidence enters Forge, and
//! it is deliberately narrow in the opposite direction from
//! [`crate::portfolio::share`]: share decides what may leave Forge,
//! this decides what may *enter* it. Five rules hold at all times.
//!
//! - **Aggregates only.** A snapshot carries allowlisted
//!   [`InterestMetric`] counts. There is no identity, no raw event, no
//!   payment record and no arbitrary metadata map: the record's key
//!   set is closed, and a key that is not on it is refused.
//! - **Provenance or nothing.** Every accepted snapshot names its
//!   project, its UTC half-open window, the source system, the source
//!   revision, how the source suppressed (or did not suppress) small
//!   cohorts, and who imported it. Forge stores what somebody else
//!   measured; it never claims to have measured it.
//! - **Immutable and idempotent.** An accepted snapshot is never
//!   rewritten. An exact duplicate is an idempotent success, and a
//!   *different* payload under an identity the source already attested
//!   to is a conflict rather than an edit.
//! - **Windows never merge.** Two same-source windows may not overlap
//!   unless the source declares a replacement, and a comparison never
//!   sums, ranks or totals across windows.
//! - **Stale is never current.** Freshness is derived at read time
//!   from the window end, and a stale row keeps its value and gains a
//!   label — it is never dropped, refreshed or silently presented as
//!   the present.
//!
//! Raw events, visitor identities, payment records and product
//! databases stay inside the analytics provider or the product. There
//! is no collector, no pixel and no product-side script in this
//! package, and `paid_interest_events` is an aggregate signal — not a
//! payment record, and never a basis for granting access.
//!
//! The module stores nothing (persistence lives in
//! [`crate::registry::interest`]). The concerns are split so each file
//! answers one question:
//!
//! - this module — what a snapshot *is*: the bounds, the
//!   refused-key classes and the record shapes.
//! - [`vocabulary`] — the closed vocabularies every snapshot is
//!   built from: [`PrivacyMode`], [`Coverage`], [`InterestMetric`]
//!   and [`SnapshotState`].
//! - [`validation`] — what may enter Forge: the closed key set, the
//!   metric allowlist, the UTC window rules and the one
//!   [`validate_snapshot`] gate every transport goes through.
//! - [`compare`] — how evidence is read back: per-window rows, the
//!   freshness label and the refusal to sum across windows.
//! - [`activation`] — whether the evidence justifies the
//!   product-owned activation follow-up: a read-only readiness
//!   verdict with a refusal-first reason vocabulary.
//!
//! Orchestration — import and comparison — lives in
//! [`crate::portfolio::interest_report`], the single door both the CLI
//! and the JSON API enter.

pub mod activation;
pub mod compare;
pub mod validation;
pub mod vocabulary;

pub use activation::{
    build_readiness, parse_window, validate_threshold, ActivationReport, NotReadyReason, Readiness,
    ReadinessEvidence, ReadinessReason, ReadinessVerdict, RequestedWindow,
    ACTIVATION_CONTRACT_VERSION,
};
pub use compare::{
    bound_stale_after_days, build_comparison, build_trend, freshness_label, Comparison,
    ComparisonRow, Freshness, Trend, TrendPoint,
};
pub use validation::{
    classify_refused_key, looks_like_secret, parse_import_document, validate_snapshot,
};
pub use vocabulary::{Coverage, InterestMetric, PrivacyMode, SnapshotState};

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::policy::redact_credentials;

pub use chrono::SecondsFormat;

/// Contract version of the import envelope and every projection this
/// package emits. Bumping it is a breaking change for adapters.
pub const INTEREST_CONTRACT_VERSION: &str = "forge-portfolio-interest/0.1.0";

/// Upper bound on the records in one import document. A runaway or
/// hostile importer is refused before a single row is written.
pub const MAX_SNAPSHOTS_PER_IMPORT: usize = 500;

/// Upper bound on a whole import document, in bytes.
pub const MAX_IMPORT_BYTES: usize = 1024 * 1024;

/// Upper bound on an analytics source name, in characters.
pub const MAX_SOURCE_CHARS: usize = 128;

/// Upper bound on a source revision, in characters.
pub const MAX_SOURCE_REVISION_CHARS: usize = 128;

/// Upper bound on a single windowed count. A windowed visitor count
/// above this is a unit error — milliseconds reported as people — so
/// the record is refused rather than stored as an absurd figure.
pub const MAX_METRIC_VALUE: u64 = 1_000_000_000;

/// Longest window Forge accepts, in days. A longer window is not an
/// aggregate interest signal; it is a lifetime total.
pub const MAX_WINDOW_DAYS: i64 = 366;

/// Upper bound on the projects one comparison may span.
pub const MAX_COMPARE_PROJECTS: usize = 32;

/// Upper bound on the windows one trend may return.
pub const MAX_TREND_POINTS: usize = 120;

/// Default staleness bound, in days: a window that ended more than
/// this long ago reads as `stale`.
pub const DEFAULT_STALE_AFTER_DAYS: i64 = 14;

/// Longest staleness bound an operator may declare, in days.
pub const MAX_STALE_AFTER_DAYS: i64 = 365;

/// The closed key set of one snapshot record. There is no arbitrary
/// metadata map: a field that is not named here is refused, which is
/// what keeps an identity field, a raw event or a payment record out
/// of the store by construction rather than by review.
pub const SNAPSHOT_KEYS: [&str; 9] = [
    "coverage",
    "metrics",
    "privacy_mode",
    "project_id",
    "replaces_source_revision",
    "source",
    "source_revision",
    "window_end",
    "window_start",
];

/// Field names that carry a visitor identity. Matched case-insensitively
/// against a whole key, so `email_address` and `Email` are refused
/// while `emails_sent` is not.
pub const IDENTITY_KEYS: [&str; 22] = [
    "account_id",
    "client_id",
    "cookie",
    "customer_id",
    "device_id",
    "email",
    "email_address",
    "full_name",
    "ip",
    "ip_address",
    "login",
    "name",
    "phone",
    "session_id",
    "user",
    "user_agent",
    "user_id",
    "userid",
    "username",
    "visitor_id",
    "visitor_ip",
    "visitor_name",
];

/// Field names that carry a raw or per-event payload rather than an
/// aggregate count.
pub const RAW_EVENT_KEYS: [&str; 12] = [
    "click_path",
    "event",
    "event_name",
    "events",
    "page_path",
    "page_url",
    "raw_event",
    "raw_events",
    "raw_payload",
    "referrer",
    "session_events",
    "url",
];

/// Field names that carry a payment or revenue record. `paid_
/// interest_events` is an aggregate signal and is explicitly *not* on
/// this list; a real payment row is.
pub const PAYMENT_KEYS: [&str; 18] = [
    "account_number",
    "amount",
    "amount_cents",
    "bank_account",
    "card",
    "card_last4",
    "card_number",
    "charge_id",
    "currency",
    "invoice",
    "invoice_id",
    "order_id",
    "payment",
    "payment_method",
    "price",
    "revenue",
    "subscription_id",
    "transaction_id",
];

/// Field names that carry a credential.
pub const CREDENTIAL_KEYS: [&str; 14] = [
    "api_key",
    "apikey",
    "auth",
    "auth_token",
    "authorization",
    "bearer",
    "cookie_header",
    "credential",
    "credentials",
    "password",
    "private_key",
    "secret",
    "session_token",
    "token",
];

/// Well-known credential prefixes, reused from the share package's
/// detector rather than re-implemented.
pub(crate) const SECRET_PREFIXES: [&str; 11] = [
    "AKIA",
    "ASIA",
    "ghp_",
    "gho_",
    "ghu_",
    "ghs_",
    "github_pat_",
    "glpat-",
    "xoxb-",
    "xoxp-",
    "eyJ",
];

// --- refusals -----------------------------------------------------------

/// One refusal produced by the single gate.
///
/// The code and the detail are kept apart on purpose. The detail
/// explains the rule to a human; the code is what an importer can
/// branch on without parsing prose, and it is what ends up in the
/// persisted finding. Neither ever carries the offending value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotRefusal {
    pub code: &'static str,
    pub detail: String,
}

impl SnapshotRefusal {
    pub fn new(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            // Defense in depth: a refusal is rendered in an audit
            // trail and returned over HTTP, so it is scrubbed here as
            // well as at the finding boundary.
            detail: redact_interest_text(&detail.into()),
        }
    }
}

impl std::fmt::Display for SnapshotRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.detail)
    }
}

impl From<SnapshotRefusal> for String {
    fn from(refusal: SnapshotRefusal) -> Self {
        refusal.detail
    }
}

/// Refusal codes the gate can produce. They are the stable machine
/// vocabulary an importer sees, so they are named once here rather than
/// spelled inline at each refusal site.
pub mod refusal {
    pub const IDENTITY: &str = "interest-identity-refused";
    pub const RAW_EVENT: &str = "interest-raw-event-refused";
    pub const PAYMENT: &str = "interest-payment-refused";
    pub const CREDENTIAL: &str = "interest-credential-refused";
    pub const FIELD_UNKNOWN: &str = "interest-field-unknown";
    pub const SECRET: &str = "interest-secret-refused";
    pub const PROJECT: &str = "interest-project-invalid";
    pub const PROVENANCE: &str = "interest-provenance-invalid";
    pub const WINDOW: &str = "interest-window-invalid";
    pub const PRIVACY_MODE: &str = "interest-privacy-mode-invalid";
    pub const COVERAGE: &str = "interest-coverage-invalid";
    pub const REPLACEMENT: &str = "interest-replacement-invalid";
    pub const METRIC_UNKNOWN: &str = "interest-metric-unknown";
    pub const METRIC_INVALID: &str = "interest-metric-invalid";
    pub const ZERO_UNMEASURED: &str = "interest-zero-unmeasured";
    /// A refusal from persistence rather than from the gate.
    pub const OVERLAP: &str = "interest-overlap-refused";
    pub const STORE: &str = "interest-store-refused";
}

// --- records ------------------------------------------------------------

/// One aggregate metric value inside a snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MetricValue {
    pub metric: String,
    pub value: u64,
}

/// A validated, ready-to-write interest snapshot. Every field is a
/// parsed vocabulary member or an already-normalized timestamp; the
/// only remaining gate is [`validate_snapshot`], which every transport
/// goes through.
#[derive(Debug, Clone)]
pub struct InterestWrite {
    pub project_id: String,
    pub source: String,
    pub source_revision: String,
    /// UTC, half-open window start.
    pub window_start: String,
    /// UTC, half-open window end; always later than `window_start`.
    pub window_end: String,
    pub privacy_mode: PrivacyMode,
    pub coverage: Coverage,
    /// The source revision this record replaces. `None` for an
    /// ordinary new window.
    pub replaces_source_revision: Option<String>,
    /// Metric values keyed by [`InterestMetric::label`], in canonical
    /// metric order.
    pub metrics: Vec<MetricValue>,
}

impl InterestWrite {
    /// The window as a half-open interval, for the overlap rule.
    pub fn window(&self) -> (DateTime<Utc>, DateTime<Utc>) {
        (
            parse_timestamp(&self.window_start).expect("validated window start"),
            parse_timestamp(&self.window_end).expect("validated window end"),
        )
    }

    /// The value of one metric, if the snapshot reported it.
    pub fn metric(&self, metric: InterestMetric) -> Option<u64> {
        self.metrics
            .iter()
            .find(|value| value.metric == metric.label())
            .map(|value| value.value)
    }

    /// The identity the source attested to: project, source, source
    /// revision and window. A repeat of exactly this identity with
    /// exactly these values is an idempotent success.
    pub fn identity(&self) -> (String, String, String, String, String) {
        (
            self.project_id.clone(),
            self.source.clone(),
            self.source_revision.clone(),
            self.window_start.clone(),
            self.window_end.clone(),
        )
    }
}

/// A persisted snapshot plus its metric values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InterestSnapshot {
    pub id: i64,
    pub project_id: String,
    pub source: String,
    pub source_revision: String,
    pub window_start: String,
    pub window_end: String,
    pub privacy_mode: String,
    pub coverage: String,
    pub state: SnapshotState,
    pub replaces_source_revision: Option<String>,
    pub actor: String,
    pub received_at: String,
    pub metrics: Vec<MetricValue>,
}

impl InterestSnapshot {
    /// The snapshot's freshness against a reference time. Derived at
    /// read time from the window end, never stored, so changing the
    /// staleness bound never rewrites history.
    pub fn freshness(&self, now: DateTime<Utc>, stale_after_days: i64) -> Freshness {
        freshness_label(&self.window_end, now, stale_after_days)
    }

    pub fn metric(&self, metric: InterestMetric) -> Option<u64> {
        self.metrics
            .iter()
            .find(|value| value.metric == metric.label())
            .map(|value| value.value)
    }

    /// Whether two half-open windows share any instant.
    ///
    /// Two windows are half-open precisely so this is a strict
    /// comparison: a window ending exactly where the next begins is
    /// adjacent, not overlapping, and adjacent windows may be stored
    /// side by side.
    pub fn overlaps(&self, other: &InterestWrite) -> bool {
        let (start, end) = other.window();
        self.window_start.as_str() < other.window_end.as_str()
            && other.window_start.as_str() < self.window_end.as_str()
            && start < end
    }
}

// --- import document ----------------------------------------------------

/// One record of an import document, as an unvalidated key/value map.
///
/// Transports hand over the raw object rather than a parsed struct so
/// that the *closed key set* is enforced once, in
/// [`crate::portfolio::interest::validation`], instead of being
/// silently widened by a permissive `Deserialize` derive.
pub type InterestRecord = serde_json::Map<String, serde_json::Value>;

/// One unvalidated record in an import document.
#[derive(Debug, Clone)]
pub struct RawSnapshot {
    /// Zero-based position in the document, so a refusal names the
    /// record the operator actually has open.
    pub index: usize,
    pub record: InterestRecord,
}

/// A versioned batch of already-decoded snapshot records.
#[derive(Debug, Clone)]
pub struct InterestImport {
    pub records: Vec<RawSnapshot>,
}

/// A record the import refused. The detail names the field and the
/// rule; the offending value is never echoed.
#[derive(Debug, Clone, Serialize)]
pub struct InterestRejection {
    pub index: usize,
    pub project_id: String,
    pub field: String,
    pub code: String,
    pub detail: String,
}

impl InterestRejection {
    pub fn new(
        index: usize,
        project_id: &str,
        field: &str,
        code: &str,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            index,
            project_id: project_id.to_string(),
            field: field.to_string(),
            code: code.to_string(),
            // Defense in depth: the detail is scrubbed again here so a
            // caller that built it from untrusted input cannot smuggle
            // a credential or an address into a rejection that is safe
            // to display.
            detail: redact_interest_text(&detail.into()),
        }
    }
}

/// One persisted reason a snapshot was refused. Findings are evidence,
/// not state: they record the field and the rule, never the value.
#[derive(Debug, Clone, Serialize)]
pub struct InterestFinding {
    pub project_id: String,
    pub field: String,
    pub code: String,
    pub detail: String,
    pub created_at: String,
}

impl InterestFinding {
    pub fn new(project_id: &str, field: &str, code: &str, detail: impl Into<String>) -> Self {
        Self {
            project_id: project_id.to_string(),
            field: field.to_string(),
            code: code.to_string(),
            detail: redact_interest_text(&detail.into()),
            created_at: String::new(),
        }
    }
}

// --- helpers ------------------------------------------------------------

/// Scrub a string that is about to be stored, returned or logged.
///
/// The shared policy owns the credential shapes. An address is not one
/// of them, but an audit detail is not the place for an address
/// either, so email-shaped tokens are dropped here as well. Every
/// refusal, rejection and finding passes through this on its way out,
/// which is what makes it safe to surface them without an operator
/// having to trust the caller that built them.
pub(crate) fn redact_interest_text(text: &str) -> String {
    redact_emails(&redact_credentials(text))
}

/// Replace every whitespace-delimited token that carries an `@` with a
/// domain part by the redaction marker, leaving every other byte of the
/// string — including its whitespace — untouched.
///
/// Tokens are tested whole rather than scanned for `@`, because an
/// address is recognised by its shape: `ops@example.com` is an address
/// and `a@b` is not.
fn redact_emails(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    loop {
        let token_len = rest.find(char::is_whitespace).unwrap_or(rest.len());
        let (token, tail) = rest.split_at(token_len);
        if looks_like_email(token) {
            out.push_str("[REDACTED]");
        } else {
            out.push_str(token);
        }
        if tail.is_empty() {
            return out;
        }
        let whitespace_len = tail
            .find(|c: char| !c.is_whitespace())
            .unwrap_or(tail.len());
        let (whitespace, next) = tail.split_at(whitespace_len);
        out.push_str(whitespace);
        rest = next;
    }
}

/// Whether a string value carries an email shape. Field-name matching
/// is the primary defence; this catches an address smuggled under an
/// allow-listed-looking name.
pub(crate) fn looks_like_email(value: &str) -> bool {
    if value.contains(char::is_whitespace) {
        return false;
    }
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
}

/// Parse one stored RFC 3339 timestamp. Storage only ever holds
/// normalized UTC, so a failure here means a corrupt row, and the
/// caller substitutes the conservative reading rather than guessing.
pub(crate) fn parse_timestamp(raw: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(raw)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|err| format!("`{raw}` is not an RFC 3339 timestamp: {err}"))
}

/// The canonical stored form of a timestamp: UTC, second precision,
/// `Z` suffix. Normalizing on the way in is what makes window
/// comparisons a plain lexicographic string comparison.
pub(crate) fn normalize_timestamp(field: &str, raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(format!("{field} is required"));
    }
    let parsed = DateTime::parse_from_rfc3339(trimmed)
        .map_err(|err| format!("{field} must be an RFC 3339 timestamp: {err}"))?;
    Ok(parsed
        .with_timezone(&Utc)
        .to_rfc3339_opts(SecondsFormat::Secs, true))
}

/// The metric values as a lookup keyed by label, for the duplicate
/// and comparison rules.
pub(crate) fn metric_map(values: &[MetricValue]) -> BTreeMap<String, u64> {
    values
        .iter()
        .map(|value| (value.metric.clone(), value.value))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write() -> InterestWrite {
        InterestWrite {
            project_id: "alethefy".to_string(),
            source: "github-analytics".to_string(),
            source_revision: "a1b2c3".to_string(),
            window_start: "2026-09-01T00:00:00Z".to_string(),
            window_end: "2026-09-08T00:00:00Z".to_string(),
            privacy_mode: PrivacyMode::ExactCount,
            coverage: Coverage::Complete,
            replaces_source_revision: None,
            metrics: vec![MetricValue {
                metric: InterestMetric::UniqueVisitors.label().to_string(),
                value: 120,
            }],
        }
    }

    #[test]
    fn the_vocabularies_are_closed_and_self_describing() {
        assert_eq!(
            PrivacyMode::labels(),
            vec!["exact-count", "lower-bound", "undeclared"]
        );
        assert!(PrivacyMode::parse("lower-bound").is_ok());
        assert!(PrivacyMode::parse("anonymised").is_err());
        assert!(PrivacyMode::ExactCount.is_exact());
        // An undeclared figure is never quietly treated as exact.
        assert!(!PrivacyMode::Undeclared.is_exact());
        assert!(!PrivacyMode::LowerBound.is_exact());

        assert_eq!(Coverage::labels(), vec!["complete", "partial"]);
        assert!(Coverage::parse("partial").is_ok());
        assert!(Coverage::parse("sampled").is_err());

        assert_eq!(SnapshotState::labels(), vec!["accepted", "superseded"]);
        assert!(SnapshotState::Accepted.is_current());
        assert!(!SnapshotState::Superseded.is_current());
    }

    #[test]
    fn the_metric_allowlist_is_exactly_five_counts() {
        assert_eq!(
            InterestMetric::labels(),
            vec![
                "unique_visitors",
                "completed_public_workflows",
                "returning_visitors",
                "outbound_cta_clicks",
                "paid_interest_events",
            ]
        );
        for metric in InterestMetric::ALL {
            assert!(InterestMetric::parse(metric.label()).is_ok());
            assert!(!metric.unit().is_empty());
        }
        // A duration, a currency and a rate are not aggregate counts.
        for absent in ["revenue_cents", "session_seconds", "bounce_rate", "email"] {
            assert!(
                InterestMetric::parse(absent).is_err(),
                "must refuse {absent}"
            );
        }
    }

    #[test]
    fn timestamps_normalize_to_a_sortable_utc_form() {
        assert_eq!(
            normalize_timestamp("window_start", "2026-09-01T02:00:00+02:00")
                .expect("offset normalizes"),
            "2026-09-01T00:00:00Z"
        );
        assert_eq!(
            normalize_timestamp("window_start", "  2026-09-01T00:00:00Z  ").expect("trimmed"),
            "2026-09-01T00:00:00Z"
        );
        // A naive local timestamp is refused: Forge cannot know which
        // instant the source meant.
        assert!(normalize_timestamp("window_start", "2026-09-01T00:00:00").is_err());
        assert!(normalize_timestamp("window_start", "2026-09-01").is_err());
        assert!(normalize_timestamp("window_start", "not-a-time").is_err());
        assert!(normalize_timestamp("window_start", "  ").is_err());
    }

    #[test]
    fn half_open_windows_never_overlap_at_their_boundary() {
        let first = write();
        let mut adjacent = write();
        adjacent.source_revision = "b2c3d4".to_string();
        adjacent.window_start = "2026-09-08T00:00:00Z".to_string();
        adjacent.window_end = "2026-09-15T00:00:00Z".to_string();
        let snapshot = InterestSnapshot {
            id: 1,
            project_id: first.project_id.clone(),
            source: first.source.clone(),
            source_revision: first.source_revision.clone(),
            window_start: first.window_start.clone(),
            window_end: first.window_end.clone(),
            privacy_mode: "exact-count".to_string(),
            coverage: "complete".to_string(),
            state: SnapshotState::Accepted,
            replaces_source_revision: None,
            actor: "ops".to_string(),
            received_at: "2026-09-08T00:00:00Z".to_string(),
            metrics: first.metrics.clone(),
        };
        // Two windows meeting at an instant are adjacent, not overlapping.
        assert!(!snapshot.overlaps(&adjacent));

        let mut overlapping = write();
        overlapping.source_revision = "c3d4e5".to_string();
        overlapping.window_start = "2026-09-07T00:00:00Z".to_string();
        overlapping.window_end = "2026-09-14T00:00:00Z".to_string();
        assert!(snapshot.overlaps(&overlapping));

        let mut contained = write();
        contained.source_revision = "d4e5f6".to_string();
        contained.window_start = "2026-09-02T00:00:00Z".to_string();
        contained.window_end = "2026-09-03T00:00:00Z".to_string();
        assert!(snapshot.overlaps(&contained));
    }

    #[test]
    fn a_rejection_and_a_finding_never_carry_the_offending_value() {
        let rejection = InterestRejection::new(
            3,
            "alethefy",
            "metrics.email",
            "interest-identity-refused",
            "email",
        );
        assert_eq!(rejection.index, 3);
        assert_eq!(rejection.project_id, "alethefy");
        let secret = InterestRejection::new(
            0,
            "alethefy",
            "metrics",
            "interest-secret-refused",
            "token=abcdef123456",
        );
        assert!(
            !secret.detail.contains("abcdef"),
            "the value must not be echoed: {}",
            secret.detail
        );
        // The key name is not the secret; the value behind it is.
        let finding = InterestFinding::new(
            "alethefy",
            "metrics",
            "interest-write-refused",
            "api_key=s3cr3tvalue",
        );
        assert!(!finding.detail.contains("s3cr3tvalue"), "{finding:?}");
    }

    #[test]
    fn the_detail_scrubber_drops_addresses_and_keeps_everything_else() {
        assert_eq!(
            redact_interest_text("ops@example.com was refused"),
            "[REDACTED] was refused"
        );
        assert_eq!(
            redact_interest_text("token=abcdef123456"),
            "[REDACTED]",
            "the shared policy owns credential shapes"
        );
        // A token with an `@` but no domain is not an address and must
        // survive verbatim: the audit trail needs the literal value.
        assert_eq!(redact_interest_text("a@b"), "a@b");
        assert_eq!(
            redact_interest_text("metrics `unique_visitors` must not be negative"),
            "metrics `unique_visitors` must not be negative"
        );
    }

    #[test]
    fn the_write_shape_exposes_its_window_and_metrics() {
        let candidate = write();
        let (start, end) = candidate.window();
        assert_eq!(
            start.to_rfc3339_opts(SecondsFormat::Secs, true),
            "2026-09-01T00:00:00Z"
        );
        assert!(start < end);
        assert_eq!(candidate.metric(InterestMetric::UniqueVisitors), Some(120));
        assert_eq!(candidate.metric(InterestMetric::OutboundCtaClicks), None);
        assert_eq!(candidate.identity().2, "a1b2c3");
    }
}
