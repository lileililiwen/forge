//! Local, privacy-bounded import of a `platform.idea-graduation`
//! artifact (`hypora-graduation-import`).
//!
//! Hypora validates an idea and emits one operator-selected local JSON
//! artifact. This module is the *only* way that artifact enters Forge,
//! and it is deliberately narrow in the same direction as
//! [`crate::portfolio::interest`]: it decides what may **enter** Forge.
//! Five rules hold at all times.
//!
//! - **The artifact is untrusted.** It comes from another product, so
//!   it is parsed as a closed key/value map whose key set is fixed at
//!   every level; a key that is not named is refused, never dropped.
//! - **Identity, raw events, payments and credentials never enter.**
//!   The deny lists and the value-shape checks refuse the whole import
//!   before any field is read into project state.
//! - **Evidence is provenance, never permission.** A validated
//!   experiment justifies starting a project; it does not choose a
//!   stack, scaffold, deploy, publish or approve a gate.
//! - **Only the mapped brief and allowlisted provenance persist.** The
//!   original artifact bytes and every evidence excerpt are display
//!   only; the receipt carries the brief, the source block and the
//!   evidence *count*.
//! - **Failures are atomic.** Parse, validation, choice resolution and
//!   persistence leave either a complete project or nothing.
//!
//! The concerns are split so each file answers one question:
//!
//! - this module — what the artifact *is*: the pinned contract, the
//!   closed key sets, the deny lists, the bounds and the record shapes.
//! - [`validation`] — what may enter Forge: the bounded read, the
//!   document decode and the one [`validation::validate_graduation`]
//!   gate every transport goes through.
//! - [`import`] — how a validated artifact becomes a project: the
//!   proposed identity, the minimal manifest, the receipt and the
//!   registration.

pub mod import;
pub mod validation;

pub use import::{
    adopt_graduation, build_proposal, render_preview_human, GraduationAdoption, GraduationPreview,
    GraduationProposal,
};
pub use validation::{parse_artifact, read_artifact, validate_graduation, GraduationRecord};

use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serialize;

use crate::policy::redact_credentials;

/// Forge's own contract for the preview, the receipt and the JSON
/// envelope this package emits. Distinct from the producer contract:
/// this is what Forge promises *about what it did*, not what it read.
pub const GRADUATION_CONTRACT_VERSION: &str = "forge-graduation-import/0.1.0";

/// The external producer contract family this consumer accepts.
pub const IDEA_GRADUATION_CONTRACT: &str = "platform.idea-graduation";

/// The only contract major this release understands.
pub const SUPPORTED_IDEA_GRADUATION_MAJOR: u32 = 0;

/// Accepted producer revisions. This package was authored against
/// `0.1.0`; this constant is the single line the vendoring follow-up
/// changes when the published schema is digest-pinned under
/// `contracts/`.
pub const SUPPORTED_IDEA_GRADUATION_REVISIONS: [&str; 1] = ["0.1.0"];

/// Project-root sidecar directory for the imported brief and its
/// provenance, following the `.forge/semantic/` convention.
pub const GRADUATION_DIR: &str = ".forge/graduation";

/// Upper bound on a whole artifact, in bytes. A runaway or hostile
/// artifact is refused before a single value is parsed.
pub const MAX_GRADUATION_BYTES: usize = 1024 * 1024;

/// Upper bound on a provenance field (Hypora project id, revision).
pub const MAX_PROVENANCE_CHARS: usize = 200;
/// Upper bound on the brief title.
pub const MAX_TITLE_CHARS: usize = 200;
/// Upper bound on a long-form brief field or the experiment summary.
pub const MAX_BRIEF_FIELD_CHARS: usize = 2000;
/// Upper bound on the requirements list.
pub const MAX_REQUIREMENTS: usize = 50;
/// Upper bound on one requirement.
pub const MAX_REQUIREMENT_CHARS: usize = 500;
/// Upper bound on the success-metric list.
pub const MAX_SUCCESS_METRICS: usize = 20;
/// Upper bound on a success-metric name or target.
pub const MAX_METRIC_FIELD_CHARS: usize = 200;
/// Upper bound on a success-metric window.
pub const MAX_METRIC_WINDOW_CHARS: usize = 120;
/// Upper bound on the evidence list.
pub const MAX_EVIDENCE_ITEMS: usize = 20;
/// Upper bound on an evidence kind label.
pub const MAX_EVIDENCE_KIND_CHARS: usize = 120;
/// Upper bound on an evidence excerpt.
pub const MAX_EVIDENCE_EXCERPT_CHARS: usize = 1000;
/// Upper bound on the recorded actor.
pub const MAX_ACTOR_CHARS: usize = 120;

/// The closed key set of the artifact object. There is no arbitrary
/// metadata map: a field that is not named here is refused, which keeps
/// an identity field, a raw event or a credential out by construction.
pub const ARTIFACT_KEYS: [&str; 6] = [
    "brief",
    "contract",
    "experiment",
    "graduated_at",
    "hypora_project_id",
    "hypora_revision",
];

/// The closed key set of the `brief` object.
pub const BRIEF_KEYS: [&str; 6] = [
    "audience",
    "problem",
    "requirements",
    "solution",
    "success_metrics",
    "title",
];

/// The closed key set of one success metric.
pub const METRIC_KEYS: [&str; 3] = ["name", "target", "window"];

/// The closed key set of the `experiment` object.
pub const EXPERIMENT_KEYS: [&str; 3] = ["evidence", "summary", "validated"];

/// The closed key set of one evidence entry.
pub const EVIDENCE_KEYS: [&str; 3] = ["excerpt", "kind", "observed_at"];

/// Field names that carry a participant identity.
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
    "participant_id",
    "phone",
    "session_id",
    "user",
    "user_agent",
    "user_id",
    "userid",
    "username",
    "visitor_id",
    "visitor_ip",
];

/// Field names that carry a raw or per-event payload rather than an
/// aggregate observation.
pub const RAW_EVENT_KEYS: [&str; 12] = [
    "answers",
    "click_path",
    "clickstream",
    "event",
    "events",
    "page_url",
    "raw_event",
    "raw_events",
    "raw_payload",
    "referrer",
    "responses",
    "utm",
];

/// Field names that carry a payment or revenue record.
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

/// Well-known credential prefixes, mirrored from the shared detectors
/// rather than re-implemented per shape.
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

/// Validated provenance of the artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraduationSource {
    /// The full producer discriminator, e.g. `platform.idea-graduation/0.1.0`.
    pub contract: String,
    /// The parsed major, always [`SUPPORTED_IDEA_GRADUATION_MAJOR`].
    pub major: u32,
    /// The parsed patch-level revision, in the supported allowlist.
    pub schema_revision: String,
    pub hypora_project_id: String,
    pub hypora_revision: String,
    /// Normalized UTC, RFC 3339 seconds.
    pub graduated_at: String,
}

/// One success metric declared by the artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraduationSuccessMetric {
    pub name: String,
    pub target: String,
    pub window: String,
}

/// The mapped brief. This is the only artifact content that may be
/// persisted beyond provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraduationBrief {
    pub title: String,
    pub problem: String,
    pub audience: String,
    pub solution: String,
    pub requirements: Vec<String>,
    pub success_metrics: Vec<GraduationSuccessMetric>,
}

/// One aggregate evidence excerpt. Displayed in the preview and
/// deliberately **not** persisted: only the count reaches the receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraduationEvidence {
    pub kind: String,
    pub excerpt: String,
    pub observed_at: String,
}

/// The experiment block: whether the idea was validated, a bounded
/// summary and the aggregate evidence shown in the preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraduationExperiment {
    pub summary: String,
    pub validated: bool,
    pub evidence: Vec<GraduationEvidence>,
}

/// A validated artifact, ready to map. The only remaining gate is
/// [`import::adopt_graduation`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraduationImport {
    pub source: GraduationSource,
    pub brief: GraduationBrief,
    pub experiment: GraduationExperiment,
}

/// A refusal from the graduation gate. The detail names the field and
/// the rule; the offending value is never echoed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraduationRefusal {
    pub field: String,
    pub code: String,
    pub detail: String,
}

impl GraduationRefusal {
    pub fn new(
        field: impl Into<String>,
        code: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            field: field.into(),
            code: code.into(),
            detail: redact_graduation_text(&detail.into()),
        }
    }
}

impl std::fmt::Display for GraduationRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.detail)
    }
}

impl From<GraduationRefusal> for String {
    fn from(refusal: GraduationRefusal) -> Self {
        refusal.detail
    }
}

/// Refusal codes the gate can produce. They are the stable machine
/// vocabulary a producer sees, so they are named once here rather than
/// spelled inline at each refusal site.
pub mod refusal {
    pub const CONTRACT: &str = "graduation-contract-invalid";
    pub const MAJOR: &str = "graduation-major-unsupported";
    pub const REVISION: &str = "graduation-revision-unsupported";
    pub const SHAPE: &str = "graduation-shape-invalid";
    pub const FIELD_UNKNOWN: &str = "graduation-field-unknown";
    pub const IDENTITY: &str = "graduation-identity-refused";
    pub const RAW_EVENT: &str = "graduation-raw-event-refused";
    pub const PAYMENT: &str = "graduation-payment-refused";
    pub const CREDENTIAL: &str = "graduation-credential-refused";
    pub const SECRET: &str = "graduation-secret-refused";
    pub const VALUE: &str = "graduation-value-invalid";
    pub const PROVENANCE: &str = "graduation-provenance-invalid";
    pub const NOT_VALIDATED: &str = "graduation-not-validated";
    pub const BOUNDS: &str = "graduation-bound-exceeded";
}

/// Whether a value carries a credential shape. The shared policy owns
/// the `key=value` and well-known token detectors; the explicit prefix
/// scan catches the token shapes an artifact may carry without a
/// surrounding `=`.
pub(crate) fn looks_like_secret(value: &str) -> bool {
    if redact_credentials(value) != value {
        return true;
    }
    if value.contains("-----BEGIN") || value.contains("-----begin") {
        return true;
    }
    SECRET_PREFIXES.iter().any(|prefix| value.contains(prefix))
}

/// Whether a string value carries an email shape.
pub(crate) fn looks_like_email(value: &str) -> bool {
    if value.contains(char::is_whitespace) {
        return false;
    }
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
}

/// Whether a string value carries a raw URL with a query string. A
/// per-visitor referrer or landing page arrives as `https://…?utm_…`,
/// and that query is participant data by construction.
pub(crate) fn is_raw_url_with_query(value: &str) -> bool {
    let Some((scheme, rest)) = value.split_once("://") else {
        return false;
    };
    if scheme.is_empty()
        || !scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
    {
        return false;
    }
    !rest.is_empty() && rest.contains('?')
}

/// Scrub a string that is about to be stored, returned or logged.
/// Credential shapes are the shared policy's; an address is not one of
/// them but is dropped here as well, so it is safe to surface a refusal
/// without trusting the caller that built it.
pub(crate) fn redact_graduation_text(text: &str) -> String {
    redact_emails(&redact_credentials(text))
}

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

/// The canonical stored form of a timestamp: UTC, second precision,
/// `Z` suffix. A timestamp without an offset is refused rather than
/// assumed UTC.
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

/// Clean the recorded actor: bounded, control-free and scrubbed. An
/// empty actor falls back to `local-admin`.
pub(crate) fn clean_actor(actor: &str) -> String {
    let cleaned: String = actor
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_ACTOR_CHARS)
        .collect();
    if cleaned.is_empty() {
        "local-admin".to_string()
    } else {
        redact_graduation_text(&cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_contract_family_major_and_revision_are_pinned() {
        assert_eq!(IDEA_GRADUATION_CONTRACT, "platform.idea-graduation");
        assert_eq!(SUPPORTED_IDEA_GRADUATION_MAJOR, 0);
        assert_eq!(SUPPORTED_IDEA_GRADUATION_REVISIONS, ["0.1.0"]);
        assert_eq!(GRADUATION_CONTRACT_VERSION, "forge-graduation-import/0.1.0");
    }

    #[test]
    fn the_closed_key_sets_are_exact() {
        assert_eq!(ARTIFACT_KEYS.len(), 6);
        assert_eq!(BRIEF_KEYS.len(), 6);
        assert_eq!(METRIC_KEYS.len(), 3);
        assert_eq!(EXPERIMENT_KEYS.len(), 3);
        assert_eq!(EVIDENCE_KEYS.len(), 3);
        for key in [
            "participant_id",
            "email",
            "raw_events",
            "answers",
            "payment",
            "token",
            "api_key",
        ] {
            assert!(
                !ARTIFACT_KEYS.contains(&key),
                "{key} must never be an artifact key"
            );
        }
    }

    #[test]
    fn the_deny_lists_are_closed_and_disjoint() {
        for (idx, list) in [
            IDENTITY_KEYS.iter(),
            RAW_EVENT_KEYS.iter(),
            PAYMENT_KEYS.iter(),
            CREDENTIAL_KEYS.iter(),
        ]
        .into_iter()
        .enumerate()
        {
            let mut seen = std::collections::BTreeSet::new();
            for key in list {
                assert!(seen.insert(*key), "list {idx} repeats {key}");
                assert_eq!(*key, key.to_ascii_lowercase(), "{key} must be lowercase");
            }
        }
    }

    #[test]
    fn a_credential_shaped_value_is_detected() {
        assert!(looks_like_secret(
            "ghp_abcdefghijklmnopqrstuvwxyz0123456789"
        ));
        assert!(looks_like_secret("-----BEGIN RSA PRIVATE KEY-----"));
        assert!(looks_like_secret("token=abcdef123456"));
        assert!(!looks_like_secret("GPA Simulator"));
    }

    #[test]
    fn an_email_or_query_url_value_is_detected() {
        assert!(looks_like_email("ops@example.com"));
        assert!(!looks_like_email("a@b"));
        assert!(is_raw_url_with_query("https://example.com/?utm_source=x"));
        assert!(!is_raw_url_with_query("https-source"));
    }

    #[test]
    fn refusal_details_are_scrubbed() {
        let refusal = GraduationRefusal::new(
            "brief.problem",
            refusal::SECRET,
            "value ops@example.com token=ghp_abcdefghijklmnopqrstuvwxyz0123456789 was refused",
        );
        assert!(
            !refusal.detail.contains("ops@example.com"),
            "{}",
            refusal.detail
        );
        assert!(!refusal.detail.contains("ghp_"), "{}", refusal.detail);
        assert!(refusal.detail.contains("[REDACTED]"));
    }

    #[test]
    fn a_shared_secret_prefix_is_refused() {
        assert!(looks_like_secret("AKIAIOSFODNN7EXAMPLE"));
        assert!(looks_like_secret("xoxb-123456789012"));
    }

    #[test]
    fn an_actor_is_bounded_scrubbed_and_defaulted() {
        assert_eq!(clean_actor(""), "local-admin");
        assert_eq!(clean_actor("  ops-admin  "), "ops-admin");
        assert!(!clean_actor("token=ghp_abcdefghijklmnopqrstuvwxyz0123456789").contains("ghp_"));
    }

    #[test]
    fn a_timestamp_is_normalized_to_utc_seconds() {
        assert_eq!(
            normalize_timestamp("graduated_at", "2026-09-20T02:00:00+02:00").unwrap(),
            "2026-09-20T00:00:00Z"
        );
        assert!(normalize_timestamp("graduated_at", "2026-09-20").is_err());
    }

    #[test]
    fn the_revision_constant_is_the_only_pin() {
        // The vendoring follow-up changes exactly this constant and the
        // local fixtures; the family and the major are stable.
        assert_eq!(SUPPORTED_IDEA_GRADUATION_REVISIONS, ["0.1.0"]);
        assert_eq!(IDEA_GRADUATION_CONTRACT, "platform.idea-graduation");
        assert_eq!(SUPPORTED_IDEA_GRADUATION_MAJOR, 0);
        let accepted = format!(
            "{IDEA_GRADUATION_CONTRACT}/{}",
            SUPPORTED_IDEA_GRADUATION_REVISIONS[0]
        );
        assert_eq!(accepted, "platform.idea-graduation/0.1.0");
    }
}
